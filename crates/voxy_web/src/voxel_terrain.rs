//! Asynchronously generated GPU chunks supplied to the shared halo mesher.
use std::collections::BTreeMap;
use voxy_core::{CancelToken, ChunkPos};
use voxy_world::{
    ChunkGenerator, GeneratedChunk, GenerationError, GeneratorDescriptor, PalettedBlocks,
    TerrainPalette, WorldSeed,
};

pub(super) struct PreparedTerrain {
    chunks: BTreeMap<ChunkPos, GeneratedChunk>,
    seed: WorldSeed,
    descriptor: GeneratorDescriptor,
}
impl PreparedTerrain {
    pub fn new(
        chunks: Vec<GeneratedChunk>,
        seed: WorldSeed,
        source: [voxy_world::BlockStateId; 5],
        target: TerrainPalette,
        water: voxy_world::BlockStateId,
    ) -> Result<Self, GenerationError> {
        let destination = [target.air, target.surface, target.soil, target.stone, water];
        let mut prepared = BTreeMap::new();
        for mut chunk in chunks {
            let mut dense = Vec::with_capacity(voxy_core::CHUNK_VOLUME);
            for index in 0..32_768_u16 {
                let index = voxy_core::LocalIndex::new(index)
                    .ok_or(GenerationError::InvalidGeneratedChunk)?;
                let block = chunk.data.blocks.get(index);
                let entry = source
                    .iter()
                    .position(|id| *id == block)
                    .ok_or(GenerationError::InvalidGeneratedChunk)?;
                dense.push(destination[entry]);
            }
            chunk.data.blocks = PalettedBlocks::from_dense(dense)
                .map_err(|_| GenerationError::InvalidGeneratedChunk)?;
            if prepared.insert(chunk.pos, chunk).is_some() {
                return Err(GenerationError::InvalidGeneratedChunk);
            }
        }
        Ok(Self {
            chunks: prepared,
            seed,
            descriptor: voxy_world::ProceduralTerrainGenerator::new(target, water).descriptor(),
        })
    }
}
impl ChunkGenerator for PreparedTerrain {
    fn descriptor(&self) -> GeneratorDescriptor {
        self.descriptor.clone()
    }
    fn generate(
        &self,
        pos: ChunkPos,
        seed: WorldSeed,
        cancel: &CancelToken,
    ) -> Result<GeneratedChunk, GenerationError> {
        if cancel.is_cancelled() {
            return Err(GenerationError::Cancelled);
        }
        if seed != self.seed {
            return Err(GenerationError::InvalidGeneratedChunk);
        }
        self.chunks
            .get(&pos)
            .cloned()
            .ok_or(GenerationError::InvalidGeneratedChunk)
    }
}

/// Checks every retained center and halo block against the CPU terrain reference.
pub(super) async fn validate_world(
    world: &voxy_world::World,
) -> Result<u32, wasm_bindgen::JsValue> {
    use super::browser::{error, yield_browser};
    use voxy_world::{ResourceKey, VoxelView};
    let registry = world.registry();
    let find = |name: &str| -> Result<voxy_world::BlockStateId, wasm_bindgen::JsValue> {
        registry
            .find(&ResourceKey::parse(format!("voxy:{name}")).map_err(error)?)
            .ok_or_else(|| error("missing scene terrain block"))
    };
    let generator = voxy_world::ProceduralTerrainGenerator::new(
        TerrainPalette {
            air: find("air")?,
            surface: find("grass")?,
            soil: find("dirt")?,
            stone: find("stone")?,
        },
        find("water_8")?,
    );
    let mut compared = 0;
    for y in -1..=1 {
        for z in -1..=1 {
            for x in -1..=1 {
                let pos = ChunkPos { x, y, z };
                let actual = world
                    .chunk(pos)
                    .ok_or_else(|| error("missing scene halo chunk"))?;
                let expected = generator
                    .generate(pos, WorldSeed(42), &CancelToken::new())
                    .map_err(error)?;
                for index in 0..32_768_u16 {
                    let index = voxy_core::LocalIndex::new(index)
                        .ok_or_else(|| error("invalid terrain index"))?;
                    if actual.data.blocks.get(index) != expected.data.blocks.get(index) {
                        return Err(error(format!(
                            "scene terrain mismatch at {pos:?}, {index:?}"
                        )));
                    }
                    compared += 1;
                }
                yield_browser().await?;
            }
        }
    }
    Ok(compared)
}
