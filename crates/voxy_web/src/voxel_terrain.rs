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

pub(crate) async fn yield_browser() -> Result<(), wasm_bindgen::JsValue> {
    use super::browser::error;
    use wasm_bindgen::JsValue;
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        let result = web_sys::window()
            .ok_or_else(|| error("missing browser window"))
            .and_then(|window| {
                window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0)
            });
        if let Err(error) = result {
            let _ = reject.call1(&JsValue::UNDEFINED, &error);
        }
    });
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map(|_| ())
}

pub(super) fn dense_ids(
    chunk: &voxy_world::GeneratedChunk,
) -> Result<Vec<u32>, wasm_bindgen::JsValue> {
    use super::browser::error;
    let mut ids = Vec::with_capacity(voxy_core::CHUNK_VOLUME);
    for y in 0..32 {
        for z in 0..32 {
            for x in 0..32 {
                let local = voxy_core::LocalPos::new(x, y, z).map_err(error)?;
                ids.push(chunk.data.blocks.get(local.index()).get());
            }
        }
    }
    Ok(ids)
}

pub(super) async fn dispatch_terrain_chunk(
    program: Option<&voxy_gpu::TerrainProgram>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pos: voxy_core::ChunkPos,
    seed: voxy_world::WorldSeed,
) -> Result<voxy_world::GeneratedChunk, wasm_bindgen::JsValue> {
    use super::browser::error;
    let program = program.ok_or_else(|| error("terrain compute unsupported on WebGL"))?;
    let job = program
        .create_job(device, pos, seed, &voxy_core::CancelToken::new())
        .map_err(error)?;
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let dispatch = job.encode(&mut encoder).map_err(error)?;
    queue.submit([encoder.finish()]);
    let mut read = dispatch.begin_read();
    let deadline = js_sys::Date::now() + 30000.0;
    loop {
        if let Some(chunk) = read.try_read().map_err(error)? {
            return Ok(chunk);
        }
        if js_sys::Date::now() >= deadline {
            return Err(error("browser terrain readback timed out"));
        }
        yield_browser().await?;
    }
}

pub(super) async fn build_voxel_world_scene(
    program: Option<&voxy_gpu::TerrainProgram>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<voxy_runtime::BootstrapScene, wasm_bindgen::JsValue> {
    use super::browser::error;
    if program.is_none() {
        return voxy_runtime::build_procedural_scene(42, 0).map_err(error);
    }
    let seed = voxy_world::WorldSeed(42);
    let mut chunks = Vec::with_capacity(27);
    for y in -1..=1 {
        for z in -1..=1 {
            for x in -1..=1 {
                chunks.push(
                    dispatch_terrain_chunk(
                        program,
                        device,
                        queue,
                        voxy_core::ChunkPos { x, y, z },
                        seed,
                    )
                    .await?,
                );
            }
        }
    }
    let (palette, water) = terrain_palette().map_err(error)?;
    let source = [
        palette.air,
        palette.surface,
        palette.soil,
        palette.stone,
        water,
    ];
    voxy_runtime::build_generated_scene(42, 0, |target, water| {
        Ok(Box::new(PreparedTerrain::new(
            chunks, seed, source, target, water,
        )?))
    })
    .map_err(error)
}

/// Checks every retained center and halo block against the CPU terrain reference.
pub(super) async fn validate_world(
    world: &voxy_world::World,
) -> Result<u32, wasm_bindgen::JsValue> {
    use super::browser::error;
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

pub(super) fn terrain_palette()
-> Result<(voxy_world::TerrainPalette, voxy_world::BlockStateId), voxy_world::RegistryError> {
    use voxy_world::{
        BlockDef, BlockRegistry, CollisionShape, MaterialId, Occlusion, RenderKind, ResourceKey,
        TerrainPalette,
    };
    let definitions = ["air", "surface", "soil", "stone", "water"]
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            Ok(BlockDef {
                key: ResourceKey::parse(format!("voxy:{name}"))?,
                render: if i == 0 {
                    RenderKind::Invisible
                } else {
                    RenderKind::Opaque
                },
                occlusion: if i == 0 {
                    Occlusion::None
                } else {
                    Occlusion::FullCube
                },
                collision: if i == 0 {
                    CollisionShape::Empty
                } else {
                    CollisionShape::FullCube
                },
                face_materials: [MaterialId(0); 6],
                translucent_interface_group: None,
                emission: 0,
                blast_resistance: 0,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let registry = BlockRegistry::new(definitions)?;
    let find = |name| {
        registry
            .find(&ResourceKey::parse(format!("voxy:{name}"))?)
            .ok_or(voxy_world::RegistryError::MissingAir)
    };
    Ok((
        TerrainPalette {
            air: find("air")?,
            surface: find("surface")?,
            soil: find("soil")?,
            stone: find("stone")?,
        },
        find("water")?,
    ))
}

pub(super) fn voxel_camera(
    scene: Option<&voxy_runtime::BootstrapScene>,
    aspect: f32,
) -> Result<(glam::Vec3, glam::Vec3, f32), wasm_bindgen::JsValue> {
    use super::browser::error;
    use glam::Vec3;
    let Some(scene) = scene else {
        return Ok((Vec3::new(0.0, 1.0, 4.0), Vec3::ZERO, 100.0));
    };
    if scene.chunks.len() <= 1 {
        return Ok((Vec3::new(1.8, 2.2, 2.6), Vec3::ZERO, 100.0));
    }
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for chunk in &scene.chunks {
        let relative = |value: i64, anchor: i64| -> Result<f32, wasm_bindgen::JsValue> {
            let delta = value
                .checked_sub(anchor)
                .ok_or_else(|| error("camera offset overflow"))?;
            Ok(f32::from(i16::try_from(delta).map_err(error)?) * 2.0)
        };
        let origin = Vec3::new(
            relative(chunk.pos.x, scene.anchor.x)?,
            relative(chunk.pos.y, scene.anchor.y)?,
            relative(chunk.pos.z, scene.anchor.z)?,
        );
        let origin = origin - Vec3::new(1.0, 0.625, 1.0);
        min = min.min(origin);
        max = max.max(origin + Vec3::splat(2.0));
    }
    let target = (min + max) * 0.5;
    if !aspect.is_finite() || aspect <= 0.0 {
        return Err(error("invalid camera aspect"));
    }
    let radius = (max - min).length() * 0.5;
    let half_fov = 27.5_f32.to_radians();
    let limiting_fov = half_fov.min((half_fov.tan() * aspect).atan());
    let distance = radius / limiting_fov.sin() * 1.05;
    let far = (distance + radius * 1.2).max(100.0);
    if !distance.is_finite() || !far.is_finite() {
        return Err(error("camera range overflow"));
    }
    Ok((
        target + Vec3::new(1.8, 2.2, 2.6).normalize() * distance,
        target,
        far,
    ))
}
