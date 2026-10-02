use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use voxy_core::{CHUNK_VOLUME, CancelToken, ChunkPos, LocalPos, VoxelPos, join_voxel};

use crate::{BlockStateId, ChunkData, PalettedBlocks};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorldSeed(pub u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratorDescriptor {
    pub id: Arc<str>,
    pub version: u32,
    pub parameters_hash: u64,
}

#[derive(Clone, Debug)]
pub struct GeneratedChunk {
    pub pos: ChunkPos,
    pub data: ChunkData,
}

pub trait ChunkGenerator: Send + Sync {
    fn descriptor(&self) -> GeneratorDescriptor;

    /// Generates one chunk as a pure function of coordinates, seed, and descriptor.
    ///
    /// # Errors
    ///
    /// Returns cancellation, coordinate overflow, or invalid generated data.
    fn generate(
        &self,
        pos: ChunkPos,
        seed: WorldSeed,
        cancel: &CancelToken,
    ) -> Result<GeneratedChunk, GenerationError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerrainPalette {
    pub air: BlockStateId,
    pub surface: BlockStateId,
    pub soil: BlockStateId,
    pub stone: BlockStateId,
}

#[derive(Clone, Debug)]
pub struct SimpleTerrainGenerator {
    palette: TerrainPalette,
    base_height: i64,
    relief: u8,
}

impl SimpleTerrainGenerator {
    #[must_use]
    pub fn new(palette: TerrainPalette, base_height: i64, relief: u8) -> Self {
        Self {
            palette,
            base_height,
            relief,
        }
    }

    fn height(&self, x: i64, z: i64, seed: WorldSeed) -> i64 {
        if self.relief == 0 {
            return self.base_height;
        }
        let phase_x = i64::from(u8::try_from(seed.0 & 31).unwrap_or(0));
        let phase_z = i64::from(u8::try_from((seed.0 >> 8) & 31).unwrap_or(0));
        let ridge_x = 12 - (x + phase_x).rem_euclid(24);
        let ridge_z = 12 - (z + phase_z).rem_euclid(24);
        let ridge = i64::midpoint(ridge_x.abs(), ridge_z.abs());
        let height = i64::from(self.relief).saturating_sub(ridge);
        self.base_height.saturating_add(height.max(0))
    }
}

impl ChunkGenerator for SimpleTerrainGenerator {
    fn descriptor(&self) -> GeneratorDescriptor {
        GeneratorDescriptor {
            id: Arc::from("voxy:simple_terrain"),
            version: 1,
            parameters_hash: splitmix64(
                u64::from_ne_bytes(self.base_height.to_ne_bytes()) ^ u64::from(self.relief),
            ),
        }
    }

    fn generate(
        &self,
        pos: ChunkPos,
        seed: WorldSeed,
        cancel: &CancelToken,
    ) -> Result<GeneratedChunk, GenerationError> {
        let mut dense = vec![self.palette.air; CHUNK_VOLUME];
        for x in 0_u8..32 {
            if cancel.is_cancelled() {
                return Err(GenerationError::Cancelled);
            }
            for z in 0_u8..32 {
                let column_origin = join_voxel(pos, LocalPos::new(x, 0, z).expect("loop bounds"))
                    .map_err(|_| GenerationError::CoordinateOverflow)?;
                let height = self.height(column_origin.x, column_origin.z, seed);
                for y in 0_u8..32 {
                    let local = LocalPos::new(x, y, z).expect("loop bounds");
                    let VoxelPos { y: world_y, .. } =
                        join_voxel(pos, local).map_err(|_| GenerationError::CoordinateOverflow)?;
                    let block = if world_y > height {
                        self.palette.air
                    } else if world_y == height {
                        self.palette.surface
                    } else if world_y >= height.saturating_sub(3) {
                        self.palette.soil
                    } else {
                        self.palette.stone
                    };
                    dense[usize::from(local.index().get())] = block;
                }
            }
        }
        let blocks = PalettedBlocks::from_dense(dense)
            .map_err(|_| GenerationError::InvalidGeneratedChunk)?;
        Ok(GeneratedChunk {
            pos,
            data: ChunkData {
                blocks,
                block_data: BTreeMap::default(),
            },
        })
    }
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationError {
    Cancelled,
    CoordinateOverflow,
    InvalidGeneratedChunk,
    /// An explicitly selected generation backend failed; no fallback was applied.
    BackendFailure,
}

impl fmt::Display for GenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "generation failed: {self:?}")
    }
}

impl std::error::Error for GenerationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn generator() -> SimpleTerrainGenerator {
        SimpleTerrainGenerator::new(
            TerrainPalette {
                air: BlockStateId::AIR,
                surface: BlockStateId::from_test(1),
                soil: BlockStateId::from_test(2),
                stone: BlockStateId::from_test(3),
            },
            4,
            8,
        )
    }

    #[test]
    fn generation_is_request_order_independent() {
        let positions = [
            ChunkPos { x: -1, y: 0, z: 2 },
            ChunkPos { x: 4, y: -1, z: 0 },
            ChunkPos { x: 0, y: 0, z: 0 },
        ];
        let generator = generator();
        let token = CancelToken::new();
        let forward: Vec<_> = positions
            .iter()
            .map(|&pos| generator.generate(pos, WorldSeed(42), &token).unwrap())
            .collect();
        let mut reverse: Vec<_> = positions
            .iter()
            .rev()
            .map(|&pos| generator.generate(pos, WorldSeed(42), &token).unwrap())
            .collect();
        reverse.reverse();
        for (left, right) in forward.iter().zip(reverse) {
            assert_eq!(left.pos, right.pos);
            assert_eq!(left.data, right.data);
        }
    }

    #[test]
    fn generation_honors_cancellation() {
        let token = CancelToken::new();
        token.cancel();
        assert!(matches!(
            generator().generate(ChunkPos { x: 0, y: 0, z: 0 }, WorldSeed(1), &token),
            Err(GenerationError::Cancelled)
        ));
    }
}
