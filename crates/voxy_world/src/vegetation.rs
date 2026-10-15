//! Chunk-order independent vegetation decorating the canonical procedural terrain.
use crate::{
    Biome, BlockStateId, ChunkGenerator, GeneratedChunk, GenerationError, GeneratorDescriptor,
    ProceduralTerrainGenerator, WorldSeed,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use voxy_core::{CancelToken, ChunkPos, LocalPos, VoxelPos, join_voxel, split_voxel};

#[derive(Clone, Copy, Debug)]
pub struct VegetationPalette {
    pub air: BlockStateId,
    pub wood: BlockStateId,
    pub leaves: BlockStateId,
}

/// Adds seeded broadleaf trees and shrubs, including crowns rooted in neighboring chunks.
/// The base generator must use the same terrain profile as `terrain`.
pub struct VegetationGenerator {
    base: Box<dyn ChunkGenerator>,
    terrain: ProceduralTerrainGenerator,
    palette: VegetationPalette,
}
impl std::fmt::Debug for VegetationGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VegetationGenerator")
            .field("base", &self.base.descriptor())
            .field("terrain", &self.terrain)
            .field("palette", &self.palette)
            .finish()
    }
}
impl VegetationGenerator {
    #[must_use]
    pub fn new(
        base: Box<dyn ChunkGenerator>,
        terrain: ProceduralTerrainGenerator,
        palette: VegetationPalette,
    ) -> Self {
        Self {
            base,
            terrain,
            palette,
        }
    }
}
fn hash(mut n: u64) -> u64 {
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    n ^ (n >> 31)
}
impl ChunkGenerator for VegetationGenerator {
    fn descriptor(&self) -> GeneratorDescriptor {
        let base = self.base.descriptor();
        let mut parameters_hash = hash(base.parameters_hash ^ u64::from(base.version));
        for b in base.id.bytes() {
            parameters_hash = hash(parameters_hash ^ u64::from(b));
        }
        parameters_hash = hash(parameters_hash ^ self.terrain.descriptor().parameters_hash);
        for b in [self.palette.air, self.palette.wood, self.palette.leaves] {
            parameters_hash = hash(parameters_hash ^ u64::from(b.get()));
        }
        GeneratorDescriptor {
            id: Arc::from("voxy:vegetated_terrain"),
            version: 2,
            parameters_hash,
        }
    }
    fn generate(
        &self,
        pos: ChunkPos,
        seed: WorldSeed,
        cancel: &CancelToken,
    ) -> Result<GeneratedChunk, GenerationError> {
        let mut chunk = self.base.generate(pos, seed, cancel)?;
        let origin = join_voxel(pos, LocalPos::new(0, 0, 0).expect("origin"))
            .map_err(|_| GenerationError::CoordinateOverflow)?;
        let mut updates = BTreeMap::new();
        // One candidate per 4x4 cell; a three-voxel halo covers every crown.
        for cx in
            (origin.x.saturating_sub(3)).div_euclid(4)..=(origin.x.saturating_add(34)).div_euclid(4)
        {
            if cancel.is_cancelled() {
                return Err(GenerationError::Cancelled);
            }
            for cz in (origin.z.saturating_sub(3)).div_euclid(4)
                ..=(origin.z.saturating_add(34)).div_euclid(4)
            {
                let h = hash(seed.0 ^ hash(cx as u64) ^ hash((cz as u64) ^ 0x763e_91a5));
                if h % 5 > 2 {
                    continue;
                }
                let Some(x) = cx
                    .checked_mul(4)
                    .and_then(|n| n.checked_add(((h >> 8) % 4) as i64))
                else {
                    continue;
                };
                let Some(z) = cz
                    .checked_mul(4)
                    .and_then(|n| n.checked_add(((h >> 16) % 4) as i64))
                else {
                    continue;
                };
                let ground = self.terrain.column(x, z, seed);
                if ground.biome != Biome::Plains
                    || ground.height <= ProceduralTerrainGenerator::SEA_LEVEL
                {
                    continue;
                }
                let tree = h % 5 == 0;
                let height = if tree { 4 + ((h >> 24) % 3) as i64 } else { 1 };
                let radius: i64 = if tree { 3 } else { 2 };
                let mut place = |dx: i64, y: i64, dz: i64, block| {
                    let (Some(x), Some(z)) = (x.checked_add(dx), z.checked_add(dz)) else {
                        return;
                    };
                    let (owner, local) = split_voxel(VoxelPos { x, y, z });
                    if owner == pos && chunk.data.blocks.get(local.index()) == self.palette.air {
                        let value = updates.entry(local.index()).or_insert(block);
                        if block == self.palette.wood {
                            *value = block;
                        }
                    }
                };
                for dy in -1_i64..=2 {
                    for dx in -radius..=radius {
                        for dz in -radius..=radius {
                            if dx * dx + dz * dz + dy * dy * 2 <= radius * radius {
                                place(dx, ground.height + height + dy + 1, dz, self.palette.leaves);
                            }
                        }
                    }
                }
                // Wood wins over leaves where the trunk passes through its crown.
                for dy in 1..=height {
                    place(0, ground.height + dy, 0, self.palette.wood);
                }
            }
        }
        chunk.data.blocks = chunk
            .data
            .blocks
            .with_updates(&updates.into_iter().collect::<Vec<_>>())
            .map_err(|_| GenerationError::InvalidGeneratedChunk)?;
        Ok(chunk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TerrainPalette;
    fn generator() -> VegetationGenerator {
        let palette = TerrainPalette {
            air: BlockStateId::AIR,
            surface: BlockStateId::from_test(1),
            soil: BlockStateId::from_test(2),
            stone: BlockStateId::from_test(3),
        };
        let terrain = ProceduralTerrainGenerator::new(palette, BlockStateId::from_test(4));
        VegetationGenerator::new(
            Box::new(terrain.clone()),
            terrain,
            VegetationPalette {
                air: palette.air,
                wood: BlockStateId::from_test(5),
                leaves: BlockStateId::from_test(6),
            },
        )
    }
    #[test]
    fn seeded_plants_cross_chunks_without_changing_ground() {
        let g = generator();
        let token = CancelToken::new();
        let mut wood = 0;
        let mut leaves = 0;
        let mut changed = false;
        let mut crown_on_boundary = false;
        for x in -2..=2 {
            for z in -2..=2 {
                let pos = ChunkPos { x, y: 0, z };
                let first = g.generate(pos, WorldSeed(42), &token).unwrap();
                let other = g.generate(pos, WorldSeed(43), &token).unwrap();
                changed |= first.data != other.data;
                assert_eq!(
                    first.data,
                    g.generate(pos, WorldSeed(42), &token).unwrap().data
                );
                let base = g.base.generate(pos, WorldSeed(42), &token).unwrap();
                for lx in 0..32 {
                    for lz in 0..32 {
                        for ly in 0..32 {
                            let local = LocalPos::new(lx, ly, lz).unwrap();
                            let world = join_voxel(pos, local).unwrap();
                            let b = first.data.blocks.get(local.index());
                            if base.data.blocks.get(local.index()) != g.palette.air {
                                assert_eq!(b, base.data.blocks.get(local.index()));
                            }
                            if b == g.palette.wood {
                                wood += 1;
                            }
                            if b == g.palette.leaves {
                                leaves += 1;
                                crown_on_boundary |= lx == 0 || lx == 31 || lz == 0 || lz == 31;
                                assert!(
                                    world.y
                                        > g.terrain.column(world.x, world.z, WorldSeed(42)).height
                                );
                            }
                        }
                    }
                }
            }
        }
        assert!(wood > 0 && leaves > 0 && changed && crown_on_boundary);
    }
    #[test]
    fn cancellation_and_descriptor() {
        let g = generator();
        let token = CancelToken::new();
        token.cancel();
        assert!(matches!(
            g.generate(ChunkPos::default(), WorldSeed(1), &token),
            Err(GenerationError::Cancelled)
        ));
        assert_ne!(g.descriptor(), g.base.descriptor());
    }
}
