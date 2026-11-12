use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use voxy_core::{CancelToken, ChunkPos, LocalPos, WorldEpoch};
use voxy_lighting::{
    LightStamp, LightVolume, LightingBudget, LightingError, LightingInput, LightingScratch,
    build_light_with_neighbor_volumes, build_light_with_scratch,
};
use voxy_mesher::{ChunkMesh, HALO_OFFSETS, MeshError, MeshStamp, MeshingInput, build_mesh};
use voxy_world::{
    BlockDef, BlockRegistry, BlockStateId, ChunkData, ChunkGenerator, ChunkRevision, ChunkSnapshot,
    CollisionShape, CommitReceipt, GeneratedChunk, GenerationError, InterfaceGroupId, MaterialId,
    Occlusion, ProceduralTerrainGenerator, RegistryError, RenderKind, ResourceKey,
    SimpleTerrainGenerator, TerrainPalette, VoxelView, World, WorldLimits, WorldSeed,
};

/// Largest supported horizontal radius for generated startup scenes.
pub const MAX_BOOTSTRAP_RADIUS: i32 = 8;

#[derive(Debug)]
pub struct BootstrapScene {
    pub anchor: ChunkPos,
    pub chunks: Vec<BootstrapChunk>,
    pub world: World,
}

/// Generates additional chunks with the same backend, seed and vegetation
/// rules used to initialize the world.
pub struct BootstrapTerrainSource {
    generator: Box<dyn ChunkGenerator>,
    seed: WorldSeed,
}

impl std::fmt::Debug for BootstrapTerrainSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BootstrapTerrainSource")
            .field("generator", &self.generator.descriptor())
            .field("seed", &self.seed)
            .finish()
    }
}

impl BootstrapTerrainSource {
    /// Generate a chunk at its absolute world position.
    ///
    /// # Errors
    /// Propagates cancellation and generation errors from the original backend.
    pub fn generate(
        &self,
        pos: ChunkPos,
        cancel: &CancelToken,
    ) -> Result<GeneratedChunk, GenerationError> {
        self.generator.generate(pos, self.seed, cancel)
    }

    /// Prepare a terrain window and its halo without changing the published world.
    ///
    /// # Errors
    /// Returns invalid radius, coordinate, snapshot, generation or derivation failures.
    pub fn prepare_window(
        &self,
        world: &World,
        retained: &BTreeMap<ChunkPos, ChunkSnapshot>,
        center: ChunkPos,
        radius: i32,
        derivation_epoch: u64,
        cancel: &CancelToken,
    ) -> Result<PreparedTerrainWindow, BootstrapError> {
        self.generate_window(world, retained, center, radius, cancel)?
            .derive_cancellable(world, retained, derivation_epoch, cancel)
    }

    /// Generate only chunks absent from both the world and retained snapshots.
    ///
    /// # Errors
    /// Reports invalid coordinates, radius, cancellation, or backend failures.
    pub fn generate_window(
        &self,
        world: &World,
        retained: &BTreeMap<ChunkPos, ChunkSnapshot>,
        center: ChunkPos,
        radius: i32,
        cancel: &CancelToken,
    ) -> Result<GeneratedTerrainWindow, BootstrapError> {
        let (visible, halo) = terrain_window_positions(center, radius)?;
        let mut generated = BTreeMap::new();
        for &pos in &halo {
            if world.chunk(pos).is_none() && !retained.contains_key(&pos) {
                let chunk = self.generate(pos, cancel)?;
                if chunk.pos != pos {
                    return Err(GenerationError::InvalidGeneratedChunk.into());
                }
                generated.insert(pos, chunk);
            }
        }
        Ok(GeneratedTerrainWindow {
            visible,
            halo,
            generated,
        })
    }
}

pub(crate) fn terrain_window_positions(
    center: ChunkPos,
    radius: i32,
) -> Result<(Vec<ChunkPos>, BTreeSet<ChunkPos>), BootstrapError> {
    if !(0..=MAX_BOOTSTRAP_RADIUS).contains(&radius) {
        return Err(BootstrapError::InvalidRadius(radius));
    }
    let radius = i64::from(radius);
    let mut visible = Vec::new();
    let mut halo = std::collections::BTreeSet::new();
    for dy in -1..=1 {
        for dz in -(radius + 1)..=radius + 1 {
            for dx in -(radius + 1)..=radius + 1 {
                let pos = ChunkPos {
                    x: center
                        .x
                        .checked_add(dx)
                        .ok_or(GenerationError::CoordinateOverflow)?,
                    y: center
                        .y
                        .checked_add(dy)
                        .ok_or(GenerationError::CoordinateOverflow)?,
                    z: center
                        .z
                        .checked_add(dz)
                        .ok_or(GenerationError::CoordinateOverflow)?,
                };
                halo.insert(pos);
                if dy == 0 && dx.abs() <= radius && dz.abs() <= radius {
                    visible.push(pos);
                }
            }
        }
    }
    Ok((visible, halo))
}

#[derive(Debug)]
pub struct GeneratedTerrainWindow {
    visible: Vec<ChunkPos>,
    halo: std::collections::BTreeSet<ChunkPos>,
    generated: BTreeMap<ChunkPos, GeneratedChunk>,
}

impl GeneratedTerrainWindow {
    /// Combine generated chunks with current resident data and retained edits.
    ///
    /// # Errors
    /// Rejects missing or invalid snapshots and failures during derivation.
    pub fn derive(
        &self,
        world: &World,
        retained: &BTreeMap<ChunkPos, ChunkSnapshot>,
        derivation_epoch: u64,
    ) -> Result<PreparedTerrainWindow, BootstrapError> {
        self.derive_reusing(world, retained, derivation_epoch, &BTreeMap::new())
    }

    /// Derive terrain geometry and lighting with cancellation.
    ///
    /// # Errors
    /// Rejects cancellation, invalid snapshots or failed derivation.
    pub fn derive_cancellable(
        &self,
        world: &World,
        retained: &BTreeMap<ChunkPos, ChunkSnapshot>,
        derivation_epoch: u64,
        cancel: &CancelToken,
    ) -> Result<PreparedTerrainWindow, BootstrapError> {
        self.derive_reusing_cancellable(world, retained, derivation_epoch, &BTreeMap::new(), cancel)
    }

    /// Reuse local meshes and lighting when the center and entire halo have the same revisions.
    /// `existing` must contain results from the same world and registry.
    ///
    /// # Errors
    /// Rejects missing or invalid snapshots and failures during derivation.
    pub fn derive_reusing(
        &self,
        world: &World,
        retained: &BTreeMap<ChunkPos, ChunkSnapshot>,
        derivation_epoch: u64,
        existing: &BTreeMap<ChunkPos, BootstrapChunk>,
    ) -> Result<PreparedTerrainWindow, BootstrapError> {
        self.derive_reusing_cancellable(
            world,
            retained,
            derivation_epoch,
            existing,
            &CancelToken::new(),
        )
    }

    /// Reuse matching geometry while allowing cancellation during derivation.
    ///
    /// # Errors
    /// Rejects cancellation, invalid snapshots or failed derivation.
    pub fn derive_reusing_cancellable(
        &self,
        world: &World,
        retained: &BTreeMap<ChunkPos, ChunkSnapshot>,
        derivation_epoch: u64,
        existing: &BTreeMap<ChunkPos, BootstrapChunk>,
        token: &CancelToken,
    ) -> Result<PreparedTerrainWindow, BootstrapError> {
        if token.is_cancelled() {
            return Err(GenerationError::Cancelled.into());
        }
        let evict = world
            .resident_chunk_positions()
            .filter(|pos| !self.halo.contains(pos))
            .collect::<Vec<_>>();
        let mut restore = Vec::new();
        let mut generate = Vec::new();
        for &pos in &self.halo {
            if world.chunk(pos).is_some() {
                continue;
            }
            if let Some(snapshot) = retained.get(&pos) {
                if snapshot.pos != pos {
                    return Err(BootstrapError::MissingGeneratedChunk);
                }
                restore.push(snapshot.clone());
            } else {
                generate.push(
                    self.generated
                        .get(&pos)
                        .cloned()
                        .ok_or(BootstrapError::MissingGeneratedChunk)?,
                );
            }
        }
        let mut world = world.clone();
        let evicted = world.replace_resident_generated_chunks(&evict, &restore, generate)?;
        let registry = world.registry_handle();
        let mut scratch = LightingScratch::new();
        let mut chunks = Vec::with_capacity(self.visible.len());
        let mut reused_chunks = 0;
        for &pos in &self.visible {
            if token.is_cancelled() {
                return Err(GenerationError::Cancelled.into());
            }
            let center = world
                .chunk(pos)
                .ok_or(BootstrapError::MissingGeneratedChunk)?;
            let mut fetch = |neighbor| world.chunk(neighbor);
            let stamp = local_mesh_stamp(pos, center.revision, derivation_epoch, &mut fetch);
            if let Some(chunk) = existing.get(&pos)
                && let Some(previous) = &chunk.local_mesh_stamp
                && previous.center == stamp.center
                && previous.halo == stamp.halo
                && previous.registry_epoch == stamp.registry_epoch
            {
                let mut chunk = chunk.clone();
                chunk.local_mesh_stamp = Some(stamp);
                chunk.light.stamp.lighting_epoch = derivation_epoch;
                chunks.push(chunk);
                reused_chunks += 1;
            } else {
                chunks.push(derive_bootstrap_chunk(
                    pos,
                    center,
                    &registry,
                    derivation_epoch,
                    token,
                    &mut scratch,
                    fetch,
                )?);
            }
        }
        chunks.sort_by_key(|chunk| chunk.pos);
        Ok(PreparedTerrainWindow {
            world,
            chunks,
            reused_chunks,
            evicted,
            restored: restore.into_iter().map(|chunk| chunk.pos).collect(),
        })
    }
}

#[derive(Debug)]
pub struct PreparedTerrainWindow {
    pub world: World,
    pub chunks: Vec<BootstrapChunk>,
    pub reused_chunks: usize,
    pub evicted: Vec<ChunkSnapshot>,
    pub restored: Vec<ChunkPos>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapChunk {
    pub pos: ChunkPos,
    pub mesh: ChunkMesh,
    pub light: LightVolume,
    local_mesh_stamp: Option<MeshStamp>,
}

/// Builds a renderer-neutral, halo-correct square of generated chunks.
///
/// # Errors
///
/// Returns registry, generation, or meshing failures without mutating global state.
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
    build_streamable_generated_scene(seed, radius, false, factory).map(|(scene, _)| scene)
}

/// Keeps seeded plant blocks as gameplay markers while rendering them through native meshes.
/// # Errors
/// Returns the same bounded bootstrap failures as `build_generated_scene`.
pub fn build_generated_scene_with_mesh_plants(
    seed: u64,
    radius: i32,
    factory: impl FnOnce(
        TerrainPalette,
        BlockStateId,
    ) -> Result<Box<dyn ChunkGenerator>, GenerationError>,
) -> Result<BootstrapScene, BootstrapError> {
    build_streamable_generated_scene(seed, radius, true, factory).map(|(scene, _)| scene)
}

/// Builds procedural terrain and retains its exact generator for later residency changes.
///
/// # Errors
/// Returns the same bounded failures as `build_generated_scene`.
pub fn build_streamable_generated_scene(
    seed: u64,
    radius: i32,
    mesh_plants: bool,
    factory: impl FnOnce(
        TerrainPalette,
        BlockStateId,
    ) -> Result<Box<dyn ChunkGenerator>, GenerationError>,
) -> Result<(BootstrapScene, BootstrapTerrainSource), BootstrapError> {
    build_scene_using_generator(seed, radius, true, mesh_plants, factory)
}

/// Builds a multi-layer vertical procedural scene across `y_min..=y_max` with halo chunks
/// `(y_min - 1)..=(y_max + 1)` and top-down + lateral cross-chunk light propagation.
///
/// # Errors
///
/// Rejects invalid radii (`!(0..=8)`), inverted or unbounded vertical spans, and generation,
/// lighting, or meshing failures.
pub fn build_vertical_procedural_scene(
    seed: u64,
    radius: i32,
    y_min: i32,
    y_max: i32,
    mesh_plants: bool,
) -> Result<BootstrapScene, BootstrapError> {
    if !(0..=MAX_BOOTSTRAP_RADIUS).contains(&radius) {
        return Err(BootstrapError::InvalidRadius(radius));
    }
    if y_min > y_max || y_max - y_min > 16 {
        return Err(BootstrapError::InvalidRadius(y_max - y_min));
    }
    let registry = Arc::new(registry_with_mesh_plants(mesh_plants)?);
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
    let generator = voxy_world::VegetationGenerator::new(
        Box::new(ProceduralTerrainGenerator::new(palette, water)),
        ProceduralTerrainGenerator::new(palette, water),
        voxy_world::VegetationPalette {
            air: palette.air,
            wood: registry
                .find(&ResourceKey::parse("voxy:wood")?)
                .ok_or(BootstrapError::MissingBuiltin)?,
            leaves: registry
                .find(&ResourceKey::parse("voxy:leaves")?)
                .ok_or(BootstrapError::MissingBuiltin)?,
        },
    );
    let radius_i64 = i64::from(radius);
    let y_lo = i64::from(y_min);
    let y_hi = i64::from(y_max);
    let mut snapshots = BTreeMap::new();
    for y in (y_lo - 1)..=(y_hi + 1) {
        for z in -(radius_i64 + 1)..=(radius_i64 + 1) {
            for x in -(radius_i64 + 1)..=(radius_i64 + 1) {
                let pos = ChunkPos { x, y, z };
                let generated = generator.generate(pos, WorldSeed(seed), &token)?;
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
    let mut positions = Vec::new();
    for y in y_lo..=y_hi {
        for z in -radius_i64..=radius_i64 {
            for x in -radius_i64..=radius_i64 {
                positions.push(ChunkPos { x, y, z });
            }
        }
    }
    let chunks = rebuild_vertical_chunks_from_snapshots(&registry, &snapshots, &positions, 1)?;
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

fn build_scene(seed: u64, radius: i32, procedural: bool) -> Result<BootstrapScene, BootstrapError> {
    build_scene_using_generator(seed, radius, procedural, false, |palette, water| {
        Ok(if procedural {
            Box::new(ProceduralTerrainGenerator::new(palette, water)) as Box<dyn ChunkGenerator>
        } else {
            Box::new(SimpleTerrainGenerator::new(palette, 7, 8)) as Box<dyn ChunkGenerator>
        })
    })
    .map(|(scene, _)| scene)
}

#[expect(
    clippy::too_many_lines,
    reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
)]
fn build_scene_using_generator(
    seed: u64,
    radius: i32,
    procedural: bool,
    mesh_plants: bool,
    factory: impl FnOnce(
        TerrainPalette,
        BlockStateId,
    ) -> Result<Box<dyn ChunkGenerator>, GenerationError>,
) -> Result<(BootstrapScene, BootstrapTerrainSource), BootstrapError> {
    if !(0..=MAX_BOOTSTRAP_RADIUS).contains(&radius) {
        return Err(BootstrapError::InvalidRadius(radius));
    }
    let registry = Arc::new(registry_with_mesh_plants(mesh_plants)?);
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
    let base = factory(palette, water)?;
    let generator: Box<dyn ChunkGenerator> = if procedural {
        Box::new(voxy_world::VegetationGenerator::new(
            base,
            ProceduralTerrainGenerator::new(palette, water),
            voxy_world::VegetationPalette {
                air: palette.air,
                wood: registry
                    .find(&ResourceKey::parse("voxy:wood")?)
                    .ok_or(BootstrapError::MissingBuiltin)?,
                leaves: registry
                    .find(&ResourceKey::parse("voxy:leaves")?)
                    .ok_or(BootstrapError::MissingBuiltin)?,
            },
        ))
    } else {
        base
    };
    let source = BootstrapTerrainSource {
        generator,
        seed: WorldSeed(seed),
    };
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
                let mut generated = source.generate(pos, &token)?;
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
                        data: Arc::new(generated.data),
                    },
                );
            }
        }
    }
    let mut chunks = Vec::with_capacity(((2 * radius + 1) * (2 * radius + 1)) as usize);
    let mut light_scratch = LightingScratch::new();
    for z in -radius..=radius {
        for x in -radius..=radius {
            let pos = ChunkPos { x, y: 0, z };
            let center = snapshots
                .get(&pos)
                .cloned()
                .ok_or(BootstrapError::MissingGeneratedChunk)?;
            chunks.push(derive_bootstrap_chunk(
                pos,
                center,
                &registry,
                1,
                &token,
                &mut light_scratch,
                |neighbor| snapshots.get(&neighbor).cloned(),
            )?);
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
    Ok((
        BootstrapScene {
            anchor: ChunkPos { x: 0, y: 0, z: 0 },
            chunks,
            world,
        },
        source,
    ))
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
    let mut light_scratch = LightingScratch::new();
    for &pos in positions {
        let center = world
            .chunk(pos)
            .ok_or(BootstrapError::MissingGeneratedChunk)?;
        chunks.push(derive_bootstrap_chunk(
            pos,
            center,
            &registry,
            derivation_epoch,
            &token,
            &mut light_scratch,
            |neighbor| world.chunk(neighbor),
        )?);
    }
    chunks.sort_by_key(|chunk| chunk.pos);
    Ok(chunks)
}

/// Extracts only `positions` and their 26-neighbor halos (`Arc`-cheap `ChunkSnapshot` clones)
/// from `world` without cloning the full `World` `BTreeMap`.
#[must_use]
pub fn extract_chunk_neighborhood(
    world: &World,
    positions: &[ChunkPos],
) -> BTreeMap<ChunkPos, ChunkSnapshot> {
    let mut snapshots = BTreeMap::new();
    for &pos in positions {
        for dz in -1..=1_i64 {
            for dy in -1..=1_i64 {
                for dx in -1..=1_i64 {
                    let (Some(x), Some(y), Some(z)) = (
                        pos.x.checked_add(dx),
                        pos.y.checked_add(dy),
                        pos.z.checked_add(dz),
                    ) else {
                        continue;
                    };
                    let target = ChunkPos { x, y, z };
                    if let std::collections::btree_map::Entry::Vacant(entry) =
                        snapshots.entry(target)
                        && let Some(snapshot) = world.chunk(target)
                    {
                        entry.insert(snapshot);
                    }
                }
            }
        }
    }
    snapshots
}

/// Rebuilds renderer-derived light and mesh data from an extracted neighborhood snapshot map.
///
/// # Errors
///
/// Returns missing chunks, lighting or meshing failures without mutating canonical state.
pub fn rebuild_chunks_from_snapshots(
    registry: &Arc<BlockRegistry>,
    snapshots: &BTreeMap<ChunkPos, ChunkSnapshot>,
    positions: &[ChunkPos],
    derivation_epoch: u64,
) -> Result<Vec<BootstrapChunk>, BootstrapError> {
    let token = CancelToken::new();
    let mut chunks = Vec::with_capacity(positions.len());
    let mut light_scratch = LightingScratch::new();
    for &pos in positions {
        let center = snapshots
            .get(&pos)
            .cloned()
            .ok_or(BootstrapError::MissingGeneratedChunk)?;
        chunks.push(derive_bootstrap_chunk(
            pos,
            center,
            registry,
            derivation_epoch,
            &token,
            &mut light_scratch,
            |neighbor| snapshots.get(&neighbor).cloned(),
        )?);
    }
    chunks.sort_by_key(|chunk| chunk.pos);
    Ok(chunks)
}

/// Rebuilds a multi-layer vertical column/grid of chunks with top-down sky propagation
/// and cross-seam neighbor `LightVolume` transfer (`build_light_with_neighbor_volumes`).
///
/// # Errors
///
/// Returns missing chunks, lighting or meshing failures.
#[expect(
    clippy::too_many_lines,
    reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
)]
pub fn rebuild_vertical_chunks_from_snapshots(
    registry: &Arc<BlockRegistry>,
    snapshots: &BTreeMap<ChunkPos, ChunkSnapshot>,
    positions: &[ChunkPos],
    derivation_epoch: u64,
) -> Result<Vec<BootstrapChunk>, BootstrapError> {
    let token = CancelToken::new();
    let mut sorted_top_down: Vec<ChunkPos> = positions.to_vec();
    sorted_top_down.sort_by(|a, b| {
        b.y.cmp(&a.y)
            .then_with(|| a.z.cmp(&b.z))
            .then_with(|| a.x.cmp(&b.x))
    });
    sorted_top_down.dedup();
    let resident_set: BTreeSet<ChunkPos> = sorted_top_down.iter().copied().collect();
    let mut light_scratch = LightingScratch::new();
    let mut solved_lights: BTreeMap<ChunkPos, LightVolume> = BTreeMap::new();

    // Pass 1 (top-down): propagate vertical skylight from highest resident `y` downward,
    // and Pass 2 (lateral): exchange skylight/blocklight across horizontal chunk seams.
    for _pass in 0..2 {
        for &pos in &sorted_top_down {
            let center = snapshots
                .get(&pos)
                .cloned()
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
            let light_neighbors = face_positions.map(|neighbor| snapshots.get(&neighbor).cloned());
            let light_faces = light_neighbors
                .each_ref()
                .map(|s| s.as_ref().map(|snap| snap.revision));
            let neighbor_lights: [Option<&LightVolume>; 6] =
                face_positions.map(|neighbor| solved_lights.get(&neighbor));
            let above = ChunkPos {
                y: pos.y + 1,
                ..pos
            };
            let sky_from_above = !resident_set.contains(&above);
            let volume = build_light_with_neighbor_volumes(
                &LightingInput {
                    center,
                    neighbors: light_neighbors,
                    registry: Arc::clone(registry),
                    sky_from_above,
                    stamp: LightStamp {
                        center: snapshots[&pos].revision,
                        faces: light_faces,
                        registry_epoch: 1,
                        lighting_epoch: derivation_epoch,
                    },
                },
                &neighbor_lights,
                LightingBudget::default(),
                &token,
                &mut light_scratch,
            )?;
            solved_lights.insert(pos, volume);
        }
    }

    let mut chunks = Vec::with_capacity(sorted_top_down.len());
    for pos in sorted_top_down {
        let center = snapshots
            .get(&pos)
            .cloned()
            .ok_or(BootstrapError::MissingGeneratedChunk)?;
        let center_revision = center.revision;
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
        let halo = neighbors
            .each_ref()
            .map(|snapshot| snapshot.as_ref().map(|s| s.revision));
        let mesh = build_mesh(
            &MeshingInput {
                center,
                neighbors,
                registry: Arc::clone(registry),
                stamp: MeshStamp {
                    center: center_revision,
                    halo,
                    registry_epoch: 1,
                    mesher_epoch: derivation_epoch,
                },
            },
            &token,
        )
        .map_err(BootstrapError::Mesh)?;
        let light = solved_lights
            .remove(&pos)
            .ok_or(BootstrapError::MissingGeneratedChunk)?;
        chunks.push(BootstrapChunk {
            pos,
            mesh,
            light,
            local_mesh_stamp: None,
        });
    }
    chunks.sort_by_key(|chunk| chunk.pos);
    Ok(chunks)
}

fn halo_position(pos: ChunkPos, index: usize) -> ChunkPos {
    let offset = HALO_OFFSETS[index];
    ChunkPos {
        x: pos.x + i64::from(offset.dx),
        y: pos.y + i64::from(offset.dy),
        z: pos.z + i64::from(offset.dz),
    }
}

fn local_mesh_stamp(
    pos: ChunkPos,
    center: ChunkRevision,
    epoch: u64,
    fetch: &mut impl FnMut(ChunkPos) -> Option<ChunkSnapshot>,
) -> MeshStamp {
    MeshStamp {
        center,
        halo: std::array::from_fn(|index| {
            fetch(halo_position(pos, index)).map(|chunk| chunk.revision)
        }),
        registry_epoch: 1,
        mesher_epoch: epoch,
    }
}

fn derive_bootstrap_chunk(
    pos: ChunkPos,
    center: ChunkSnapshot,
    registry: &Arc<BlockRegistry>,
    derivation_epoch: u64,
    token: &CancelToken,
    light_scratch: &mut LightingScratch,
    mut fetch_chunk: impl FnMut(ChunkPos) -> Option<ChunkSnapshot>,
) -> Result<BootstrapChunk, BootstrapError> {
    let center_revision = center.revision;
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
    let light_neighbors = face_positions.map(&mut fetch_chunk);
    let light_faces = light_neighbors
        .each_ref()
        .map(|snapshot| snapshot.as_ref().map(|snapshot| snapshot.revision));
    let light = build_light_with_scratch(
        &LightingInput {
            center: center.clone(),
            neighbors: light_neighbors,
            registry: Arc::clone(registry),
            sky_from_above: true,
            stamp: LightStamp {
                center: center_revision,
                faces: light_faces,
                registry_epoch: 1,
                lighting_epoch: derivation_epoch,
            },
        },
        LightingBudget::default(),
        token,
        light_scratch,
    )?;
    let neighbors = std::array::from_fn(|index| fetch_chunk(halo_position(pos, index)));
    let stamp = MeshStamp {
        center: center_revision,
        halo: neighbors
            .each_ref()
            .map(|snapshot| snapshot.as_ref().map(|snapshot| snapshot.revision)),
        registry_epoch: 1,
        mesher_epoch: derivation_epoch,
    };
    let mesh = build_mesh(
        &MeshingInput {
            center,
            neighbors,
            registry: Arc::clone(registry),
            stamp: stamp.clone(),
        },
        token,
    )
    .map_err(BootstrapError::Mesh)?;
    Ok(BootstrapChunk {
        pos,
        mesh,
        light,
        local_mesh_stamp: Some(stamp),
    })
}

/// Chunks whose derived light or mesh can differ after `receipt` was committed.
///
/// Lighting samples a one-cell halo across all six faces, so every face neighbor of an
/// edited chunk is included; meshing only crosses a boundary at edited boundary cells,
/// which the receipt already lists per chunk. Intersect the result with the resident set,
/// pass it to [`rebuild_bootstrap_chunks`] and merge the output into the previous derived
/// chunks: the merged set equals a full rebuild of every resident chunk.
#[must_use]
pub fn invalidated_derived_chunks(receipt: &CommitReceipt) -> BTreeSet<ChunkPos> {
    let mut dirty = BTreeSet::new();
    for delta in &receipt.chunks {
        dirty.insert(delta.pos);
        dirty.extend(delta.invalidated_neighbor_meshes.iter().copied());
        for (dx, dy, dz) in [
            (-1, 0, 0),
            (1, 0, 0),
            (0, -1, 0),
            (0, 1, 0),
            (0, 0, -1),
            (0, 0, 1),
        ] {
            if let (Some(x), Some(y), Some(z)) = (
                delta.pos.x.checked_add(dx),
                delta.pos.y.checked_add(dy),
                delta.pos.z.checked_add(dz),
            ) {
                dirty.insert(ChunkPos { x, y, z });
            }
        }
    }
    dirty
}

fn registry_with_mesh_plants(mesh_plants: bool) -> Result<BlockRegistry, RegistryError> {
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
    definitions.push(definition(
        "wood",
        if mesh_plants {
            RenderKind::Invisible
        } else {
            RenderKind::Opaque
        },
        if mesh_plants {
            Occlusion::None
        } else {
            Occlusion::FullCube
        },
        CollisionShape::FullCube,
        3,
    )?);
    definitions.push(definition(
        "leaves",
        if mesh_plants {
            RenderKind::Invisible
        } else {
            RenderKind::Opaque
        },
        if mesh_plants {
            Occlusion::None
        } else {
            Occlusion::FullCube
        },
        CollisionShape::Empty,
        2,
    )?);
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

#[derive(Debug, thiserror::Error)]
#[error("bootstrap scene failed: {self:?}")]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shifted_window_reuses_only_unchanged_halos() {
        let (mut scene, source) =
            build_streamable_generated_scene(19, 1, true, |palette, water| {
                Ok(Box::new(ProceduralTerrainGenerator::new(palette, water)))
            })
            .unwrap();
        let existing = scene
            .chunks
            .into_iter()
            .map(|chunk| (chunk.pos, chunk))
            .collect();
        let center = ChunkPos {
            x: 1,
            ..scene.anchor
        };
        let retained = BTreeMap::new();
        let cancel = CancelToken::new();
        let shifted = |world: &World| {
            source
                .generate_window(world, &retained, center, 1, &cancel)
                .unwrap()
                .derive_reusing(world, &retained, 2, &existing)
                .unwrap()
        };
        let unchanged = shifted(&scene.world);
        assert_eq!(unchanged.reused_chunks, 6);
        let stone = scene
            .world
            .registry()
            .find(&ResourceKey::parse("voxy:stone").unwrap())
            .unwrap();
        scene
            .world
            .commit(voxy_world::EditTxn {
                source: voxy_world::EditSource::Editor,
                expected: vec![],
                writes: vec![voxy_world::VoxelWrite {
                    pos: voxy_core::VoxelPos { x: 0, y: 31, z: 0 },
                    block: stone,
                }],
            })
            .unwrap();
        let edited = shifted(&scene.world);
        assert!(edited.reused_chunks < unchanged.reused_chunks);
        let cold = source
            .prepare_window(&scene.world, &retained, center, 1, 2, &cancel)
            .unwrap();
        assert_eq!(edited.chunks, cold.chunks);
    }

    #[test]
    fn retained_terrain_source_reproduces_bootstrap_chunks() {
        let (mut scene, source) =
            build_streamable_generated_scene(19, 0, true, |palette, water| {
                Ok(Box::new(ProceduralTerrainGenerator::new(palette, water)))
            })
            .unwrap();
        let cancel = CancelToken::new();
        for pos in [
            scene.anchor,
            ChunkPos {
                x: -1,
                ..scene.anchor
            },
        ] {
            let generated = source.generate(pos, &cancel).unwrap();
            assert_eq!(generated.pos, pos);
            assert_eq!(
                generated.data.blocks,
                scene.world.chunk(pos).unwrap().data.blocks
            );
        }
        let center = ChunkPos {
            x: 3,
            ..scene.anchor
        };
        let generated = source
            .generate_window(&scene.world, &BTreeMap::new(), center, 0, &cancel)
            .unwrap();
        let stone = scene
            .world
            .registry()
            .find(&ResourceKey::parse("voxy:stone").unwrap())
            .unwrap();
        scene
            .world
            .commit(voxy_world::EditTxn {
                source: voxy_world::EditSource::Editor,
                expected: vec![],
                writes: vec![voxy_world::VoxelWrite {
                    pos: voxy_core::VoxelPos { x: 0, y: 31, z: 0 },
                    block: stone,
                }],
            })
            .unwrap();
        let cancelled = CancelToken::new();
        cancelled.cancel();
        assert!(matches!(
            generated.derive_cancellable(&scene.world, &BTreeMap::new(), 2, &cancelled),
            Err(BootstrapError::Generation(GenerationError::Cancelled))
        ));
        let shifted = generated.derive(&scene.world, &BTreeMap::new(), 2).unwrap();
        assert_eq!(shifted.world.resident_chunk_positions().count(), 27);
        assert_eq!(shifted.chunks[0].pos, center);
        assert!(shifted.world.chunk(scene.anchor).is_none());
        let original = scene.world.chunk(scene.anchor).unwrap();
        let archive = crate::TerrainArchive::new().unwrap();
        assert!(
            archive
                .load_window(&shifted.world, scene.anchor, 0)
                .is_err()
        );
        archive
            .save_evictions(&shifted.world, &shifted.evicted)
            .unwrap();
        let retained = archive
            .load_window(&shifted.world, scene.anchor, 0)
            .unwrap();
        assert_eq!(retained.len(), 1);
        let returned = source
            .prepare_window(&shifted.world, &retained, scene.anchor, 0, 3, &cancel)
            .unwrap();
        let restored = returned.world.chunk(scene.anchor).unwrap();
        assert_eq!(restored.data.blocks, original.data.blocks);
        assert!(restored.revision > original.revision);
        assert!(returned.restored.contains(&scene.anchor));
    }

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
        for name in ["voxy:wood", "voxy:leaves"] {
            let block = first
                .world
                .registry()
                .find(&ResourceKey::parse(name).unwrap())
                .unwrap();
            assert!(
                first.chunks.iter().any(|chunk| {
                    let snapshot = first.world.chunk(chunk.pos).unwrap();
                    (0..32).any(|x| {
                        (0..32).any(|z| {
                            (0..32).any(|y| {
                                snapshot
                                    .data
                                    .blocks
                                    .get(LocalPos::new(x, y, z).unwrap().index())
                                    == block
                            })
                        })
                    })
                }),
                "startup scene is missing {name}"
            );
        }
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
    fn mesh_plants_keep_authoritative_blocks_without_duplicate_voxel_surfaces() {
        let block = build_procedural_scene(0x56_4f_58_59, 1).unwrap();
        let mesh = build_generated_scene_with_mesh_plants(0x56_4f_58_59, 1, |p, w| {
            Ok(Box::new(ProceduralTerrainGenerator::new(p, w)))
        })
        .unwrap();
        let wood = mesh
            .world
            .registry()
            .find(&ResourceKey::parse("voxy:wood").unwrap())
            .unwrap();
        let leaves = mesh
            .world
            .registry()
            .find(&ResourceKey::parse("voxy:leaves").unwrap())
            .unwrap();
        assert_eq!(
            mesh.world.registry().get(wood).unwrap().render,
            RenderKind::Invisible
        );
        assert_eq!(
            mesh.world.registry().get(wood).unwrap().collision,
            CollisionShape::FullCube
        );
        assert_eq!(
            mesh.world.registry().get(wood).unwrap().occlusion,
            Occlusion::None
        );
        assert_eq!(
            mesh.world.registry().get(leaves).unwrap().collision,
            CollisionShape::Empty
        );
        for chunk in &mesh.chunks {
            assert_eq!(
                mesh.world.chunk(chunk.pos).unwrap().data,
                block.world.chunk(chunk.pos).unwrap().data
            );
        }
        assert!(
            mesh.chunks
                .iter()
                .map(|c| c.mesh.quad_count())
                .sum::<usize>()
                < block
                    .chunks
                    .iter()
                    .map(|c| c.mesh.quad_count())
                    .sum::<usize>()
        );
    }

    #[test]
    fn rejects_unbounded_bootstrap_radius() {
        assert!(matches!(
            build_bootstrap_scene(1, 9),
            Err(BootstrapError::InvalidRadius(9))
        ));
    }

    #[test]
    fn invalidated_chunks_rebuild_equals_full_rebuild() {
        use voxy_core::VoxelPos;
        use voxy_world::{EditSource, EditTxn, VoxelWrite};

        let scene = build_procedural_scene(7, 1).unwrap();
        let mut world = scene.world;
        let positions: Vec<ChunkPos> = scene.chunks.iter().map(|chunk| chunk.pos).collect();
        let mut derived: BTreeMap<ChunkPos, BootstrapChunk> = scene
            .chunks
            .into_iter()
            .map(|chunk| (chunk.pos, chunk))
            .collect();
        let stone = world
            .registry()
            .find(&ResourceKey::parse("voxy:stone").unwrap())
            .unwrap();
        let edits: Vec<Vec<VoxelPos>> = vec![
            vec![VoxelPos {
                x: 16,
                y: 16,
                z: 16,
            }],
            vec![VoxelPos { x: 0, y: 5, z: 10 }],
            vec![VoxelPos {
                x: 31,
                y: 31,
                z: 31,
            }],
            vec![
                VoxelPos { x: -1, y: 3, z: 0 },
                VoxelPos { x: 0, y: 3, z: 0 },
            ],
            (0..40)
                .map(|i| VoxelPos {
                    x: 10 + i % 20,
                    y: 12,
                    z: 31 + i / 20,
                })
                .collect(),
            vec![VoxelPos {
                x: -32,
                y: 0,
                z: -32,
            }],
        ];
        for (round, edit) in edits.iter().enumerate() {
            let block = if round % 2 == 0 {
                stone
            } else {
                BlockStateId::AIR
            };
            let receipt = world
                .commit(EditTxn {
                    source: EditSource::Simulation,
                    expected: Vec::new(),
                    writes: edit.iter().map(|&pos| VoxelWrite { pos, block }).collect(),
                })
                .unwrap();
            let dirty: Vec<ChunkPos> = invalidated_derived_chunks(&receipt)
                .into_iter()
                .filter(|pos| positions.contains(pos))
                .collect();
            assert!(!dirty.is_empty(), "round {round}");
            assert!(dirty.len() < positions.len(), "round {round}");
            for chunk in rebuild_bootstrap_chunks(&world, &dirty, 2).unwrap() {
                derived.insert(chunk.pos, chunk);
            }
            for chunk in rebuild_bootstrap_chunks(&world, &positions, 2).unwrap() {
                let cached = &derived[&chunk.pos];
                assert_eq!(cached.mesh, chunk.mesh, "round {round} {:?}", chunk.pos);
                assert_eq!(
                    cached.light.bytes(),
                    chunk.light.bytes(),
                    "round {round} {:?}",
                    chunk.pos
                );
            }
        }
    }
}
