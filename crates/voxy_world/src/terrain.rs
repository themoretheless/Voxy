//! Seeded terrain sampled in world coordinates, independently of chunk order.
use std::collections::BTreeMap;
use std::sync::Arc;

use voxy_core::{CHUNK_VOLUME, CancelToken, ChunkPos, LocalPos, join_voxel};

use crate::{
    BlockStateId, ChunkData, ChunkGenerator, GeneratedChunk, GenerationError, GeneratorDescriptor,
    PalettedBlocks, TerrainPalette, WorldSeed,
};

const SCALE: i64 = 65_536;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Biome {
    Ocean,
    Plains,
    Mountains,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerrainColumn {
    pub height: i64,
    pub biome: Biome,
}

/// Versioned block terrain with a sea level at y=6 and heights in 0..=31.
#[derive(Clone, Debug)]
pub struct ProceduralTerrainGenerator {
    palette: TerrainPalette,
    water: BlockStateId,
}

impl ProceduralTerrainGenerator {
    pub const SEA_LEVEL: i64 = 6;

    #[must_use]
    pub fn new(palette: TerrainPalette, water: BlockStateId) -> Self {
        Self { palette, water }
    }

    #[must_use]
    pub fn column(&self, x: i64, z: i64, seed: WorldSeed) -> TerrainColumn {
        let continent = noise(x, z, 128, seed.0);
        let mountains = noise(x, z, 192, seed.0 ^ 0x92a7)
            .saturating_sub(SCALE / 2)
            .max(0)
            * 2;
        let detail = noise(x, z, 32, seed.0 ^ 0x7fc3);
        let fine = noise(x, z, 8, seed.0 ^ 0xc421);
        let height = (continent * 10 + mountains * detail * 16 / SCALE + detail * 4 + fine) / SCALE;
        let biome = if height < Self::SEA_LEVEL {
            Biome::Ocean
        } else if mountains > SCALE / 3 {
            Biome::Mountains
        } else {
            Biome::Plains
        };
        TerrainColumn { height, biome }
    }

    fn block(&self, column: TerrainColumn, y: i64) -> BlockStateId {
        if y > column.height {
            if y <= Self::SEA_LEVEL {
                self.water
            } else {
                self.palette.air
            }
        } else if column.biome == Biome::Mountains && column.height > 18 {
            self.palette.stone
        } else if y == column.height && column.biome != Biome::Ocean {
            self.palette.surface
        } else if y >= column.height - 3 {
            self.palette.soil
        } else {
            self.palette.stone
        }
    }
}

impl ChunkGenerator for ProceduralTerrainGenerator {
    fn descriptor(&self) -> GeneratorDescriptor {
        let mut hash = 0x7072_6f63_7465_7272;
        for block in [
            self.palette.air,
            self.palette.surface,
            self.palette.soil,
            self.palette.stone,
            self.water,
        ] {
            hash = mix(hash ^ u64::from(block.get()));
        }
        GeneratorDescriptor {
            id: Arc::from("voxy:procedural_terrain"),
            version: 1,
            parameters_hash: hash,
        }
    }

    fn generate(
        &self,
        pos: ChunkPos,
        seed: WorldSeed,
        cancel: &CancelToken,
    ) -> Result<GeneratedChunk, GenerationError> {
        let mut dense = vec![self.palette.air; CHUNK_VOLUME];
        for x in 0..32 {
            if cancel.is_cancelled() {
                return Err(GenerationError::Cancelled);
            }
            for z in 0..32 {
                let origin = join_voxel(pos, LocalPos::new(x, 0, z).expect("loop bounds"))
                    .map_err(|_| GenerationError::CoordinateOverflow)?;
                let column = self.column(origin.x, origin.z, seed);
                for y in 0..32 {
                    let local = LocalPos::new(x, y, z).expect("loop bounds");
                    let world =
                        join_voxel(pos, local).map_err(|_| GenerationError::CoordinateOverflow)?;
                    dense[usize::from(local.index().get())] = self.block(column, world.y);
                }
            }
        }
        Ok(GeneratedChunk {
            pos,
            data: ChunkData {
                blocks: PalettedBlocks::from_dense(dense)
                    .map_err(|_| GenerationError::InvalidGeneratedChunk)?,
                block_data: BTreeMap::new(),
            },
        })
    }
}

fn mix(mut n: u64) -> u64 {
    n = n.wrapping_add(0x9e37_79b9_7f4a_7c15);
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    n ^ (n >> 31)
}

// Integer smoothstep preserves precision even near i64 coordinate limits.
fn noise(x: i64, z: i64, period: i64, seed: u64) -> i64 {
    let cell_x = x.div_euclid(period);
    let cell_z = z.div_euclid(period);
    let smooth = |v: i64| {
        let t = v * SCALE / period;
        t * t / SCALE * (3 * SCALE - 2 * t) / SCALE
    };
    let sample = |dx: i64, dz: i64| {
        let a = u64::from_le_bytes((cell_x + dx).to_le_bytes());
        let b = u64::from_le_bytes((cell_z + dz).to_le_bytes());
        i64::try_from(mix(seed ^ mix(a) ^ mix(b ^ 0x517c_c1b7)) & 0xffff).expect("16 bits")
    };
    let lerp = |a: i64, b: i64, t: i64| a + (b - a) * t / SCALE;
    lerp(
        lerp(sample(0, 0), sample(1, 0), smooth(x.rem_euclid(period))),
        lerp(sample(0, 1), sample(1, 1), smooth(x.rem_euclid(period))),
        smooth(z.rem_euclid(period)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn generator() -> ProceduralTerrainGenerator {
        ProceduralTerrainGenerator::new(
            TerrainPalette {
                air: BlockStateId::AIR,
                surface: BlockStateId::from_test(1),
                soil: BlockStateId::from_test(2),
                stone: BlockStateId::from_test(3),
            },
            BlockStateId::from_test(4),
        )
    }

    #[test]
    fn biomes_seed_variation_and_smooth_chunk_boundaries() {
        let g = generator();
        let mut biomes = [false; 3];
        let mut changed = false;
        for x in (-1024..1024).step_by(8) {
            for z in (-1024..1024).step_by(8) {
                let c = g.column(x, z, WorldSeed(42));
                biomes[match c.biome {
                    Biome::Ocean => 0,
                    Biome::Plains => 1,
                    Biome::Mountains => 2,
                }] = true;
                changed |= c != g.column(x, z, WorldSeed(43));
                assert!((0..=31).contains(&c.height));
                assert!((c.height - g.column(x + 1, z, WorldSeed(42)).height).abs() <= 2);
                assert!((c.height - g.column(x, z + 1, WorldSeed(42)).height).abs() <= 2);
            }
        }
        assert_eq!(biomes, [true; 3]);
        assert!(changed);
        for x in [i64::MIN, i64::MAX] {
            assert!((0..=31).contains(&g.column(x, x, WorldSeed(u64::MAX)).height));
        }
    }

    #[test]
    fn chunks_match_world_samples_and_request_order() {
        let g = generator();
        let token = CancelToken::new();
        let positions = [
            ChunkPos { x: -1, y: -1, z: 0 },
            ChunkPos::default(),
            ChunkPos { x: 1, y: 0, z: -1 },
        ];
        let chunks: Vec<_> = positions
            .iter()
            .map(|&p| g.generate(p, WorldSeed(42), &token).unwrap())
            .collect();
        for (pos, first) in positions.iter().zip(&chunks).rev() {
            assert_eq!(
                first.data,
                g.generate(*pos, WorldSeed(42), &token).unwrap().data
            );
            for x in 0..32 {
                for z in 0..32 {
                    for y in 0..32 {
                        let local = LocalPos::new(x, y, z).unwrap();
                        let world = join_voxel(*pos, local).unwrap();
                        assert_eq!(
                            first.data.blocks.get(local.index()),
                            g.block(g.column(world.x, world.z, WorldSeed(42)), world.y)
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn water_fills_only_above_ground_to_sea_level() {
        let g = generator();
        let ocean = TerrainColumn {
            height: -2,
            biome: Biome::Ocean,
        };
        assert_eq!(g.block(ocean, -2), g.palette.soil);
        for y in -1..=6 {
            assert_eq!(g.block(ocean, y), g.water);
        }
        assert_eq!(g.block(ocean, 7), g.palette.air);
        let token = CancelToken::new();
        token.cancel();
        assert!(matches!(
            g.generate(ChunkPos::default(), WorldSeed(1), &token),
            Err(GenerationError::Cancelled)
        ));
        assert!(matches!(
            g.generate(
                ChunkPos {
                    x: i64::MAX,
                    y: 0,
                    z: 0
                },
                WorldSeed(1),
                &CancelToken::new()
            ),
            Err(GenerationError::CoordinateOverflow)
        ));
    }
}
