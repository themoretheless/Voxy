//! Nonblocking water transactions against the retained browser gameplay world.
use super::browser::error;
use physics_voxel::{WaterBudget, WaterPlan, WaterStates};
use voxy_gpu::{PendingTerrain, PendingWaterPlan, WaterSnapshotError, WaterTransferProgram};
use voxy_world::{
    ChunkGenerator, CommitError, EditSource, ResourceKey, Sample, VoxelPos, VoxelView,
};
use wasm_bindgen::JsValue;

#[derive(Debug)]
pub(super) struct BrowserWater {
    program: WaterTransferProgram,
    states: WaterStates,
    active: Vec<VoxelPos>,
    pending: Option<PendingWaterPlan>,
    started: Option<f64>,
    loading: Option<PendingTerrain>,
    loaded: u32,
    mesh_dirty: bool,
    visible: std::collections::BTreeSet<voxy_core::ChunkPos>,
    protected: std::collections::BTreeSet<voxy_core::ChunkPos>,
    retained: std::collections::BTreeMap<voxy_core::ChunkPos, voxy_world::ChunkSnapshot>,
}
impl BrowserWater {
    pub fn extend_pause(&mut self, milliseconds: f64) -> u32 {
        let mut pending = 0;
        if let Some(started) = &mut self.started {
            *started += milliseconds;
            pending += 1;
        }
        pending
    }

    pub async fn new(device: &wgpu::Device, world: &voxy_world::World) -> Result<Self, JsValue> {
        if device.limits().max_compute_workgroups_per_dimension == 0 {
            return Err(error("gameplay water compute unsupported on WebGL"));
        }
        let mut levels = [voxy_world::BlockStateId::AIR; 8];
        for (index, level) in levels.iter_mut().enumerate() {
            *level = world
                .registry()
                .find(&ResourceKey::parse(format!("voxy:water_{}", index + 1)).map_err(error)?)
                .ok_or_else(|| error("missing gameplay water state"))?;
        }
        let states = WaterStates(levels)
            .validate(world.registry())
            .map_err(error)?;
        let active = center_water(world, levels);
        Ok(Self {
            program: WaterTransferProgram::new(device).await.map_err(error)?,
            states,
            active,
            pending: None,
            started: None,
            loading: None,
            loaded: 0,
            protected: std::collections::BTreeSet::new(),
            retained: std::collections::BTreeMap::new(),
            mesh_dirty: false,
            visible: [voxy_core::ChunkPos { x: 0, y: 0, z: 0 }]
                .into_iter()
                .collect(),
        })
    }
    pub fn seed_boundary(&mut self, world: &mut voxy_world::World) -> Result<(), JsValue> {
        use voxy_world::{BlockStateId, EditTxn, VoxelWrite};
        let pos = VoxelPos {
            x: 63,
            y: 17,
            z: 16,
        };
        let stone = world
            .registry()
            .find(&ResourceKey::parse("voxy:stone").map_err(error)?)
            .ok_or_else(|| error("missing boundary stone"))?;
        world
            .commit(EditTxn {
                source: EditSource::Player(1),
                expected: vec![],
                writes: vec![
                    VoxelWrite {
                        pos,
                        block: self.states.0[7],
                    },
                    VoxelWrite {
                        pos: VoxelPos { y: 16, ..pos },
                        block: stone,
                    },
                    VoxelWrite {
                        pos: VoxelPos { x: 62, ..pos },
                        block: BlockStateId::AIR,
                    },
                ],
            })
            .map_err(error)?;
        self.active = vec![pos];
        self.pending = None;
        self.visible.insert(voxy_core::split_voxel(pos).0);
        self.mesh_dirty = true;
        Ok(())
    }
    pub fn loaded_chunks(&self) -> u32 {
        self.loaded
    }
    pub fn needs_mesh_refresh(&self) -> bool {
        self.mesh_dirty
    }
    pub fn mark_mesh_refreshed(&mut self) {
        self.mesh_dirty = false;
    }
    pub fn visible_chunks(&self) -> Vec<voxy_core::ChunkPos> {
        self.visible.iter().copied().collect()
    }
    pub fn wake(&mut self, world: &voxy_world::World) {
        let mut active: std::collections::BTreeSet<_> = self.active.iter().copied().collect();
        for &chunk in &self.visible {
            active.extend(chunk_water(world, self.states.0, chunk));
        }
        self.active = active.into_iter().collect();
        // A completed readback from before the edit must never replace this activation set.
        self.pending = None;
    }
    pub fn protect_chunks(&mut self, chunks: std::collections::BTreeSet<voxy_core::ChunkPos>) {
        self.protected = chunks;
    }
    fn retire_chunk(
        &mut self,
        world: &mut voxy_world::World,
        pos: voxy_core::ChunkPos,
    ) -> Result<(), JsValue> {
        if self.protected.contains(&pos) {
            return Err(error("cannot retire actor collision data"));
        }
        if self.pending.is_some()
            || self.loading.is_some()
            || self
                .active
                .iter()
                .any(|&cell| voxy_core::split_voxel(cell).0 == pos)
        {
            return Err(error("cannot retire active or in-flight water data"));
        }
        let snapshot = world.unload_chunk(pos).map_err(error)?;
        self.retained.insert(pos, snapshot);
        self.visible.remove(&pos);
        self.mesh_dirty = true;
        Ok(())
    }
    fn trim_resident_chunks(&mut self, world: &mut voxy_world::World) -> Result<(), JsValue> {
        const TARGET: usize = 32;
        if self.pending.is_some() || self.loading.is_some() || self.visible.len() <= TARGET {
            return Ok(());
        }
        let mut protected = self.protected.clone();
        // Keep the bootstrap chunk and every queued water cell's neighboring chunks.
        // A one-chunk halo covers water samples and nearby actor contacts before the next query.
        protected.insert(voxy_core::ChunkPos { x: 0, y: 0, z: 0 });
        let roots: std::collections::BTreeSet<_> = self
            .protected
            .iter()
            .copied()
            .chain(
                self.active
                    .iter()
                    .map(|&cell| voxy_core::split_voxel(cell).0),
            )
            .collect();
        for chunk in roots {
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        if let (Some(x), Some(y), Some(z)) = (
                            chunk.x.checked_add(dx),
                            chunk.y.checked_add(dy),
                            chunk.z.checked_add(dz),
                        ) {
                            protected.insert(voxy_core::ChunkPos { x, y, z });
                        }
                    }
                }
            }
        }
        let excess = self.visible.len() - TARGET;
        let candidates: Vec<_> = self
            .visible
            .difference(&protected)
            .copied()
            .take(excess)
            .collect();
        for chunk in candidates {
            self.retire_chunk(world, chunk)?;
        }
        Ok(())
    }

    fn finish_batch(&mut self, next_active: &[VoxelPos]) {
        let completed = self.active.len().min(WaterBudget::default().max_active);
        self.active.drain(..completed);
        let mut queued: std::collections::BTreeSet<_> = self.active.iter().copied().collect();
        self.active.extend(
            next_active
                .iter()
                .copied()
                .filter(|pos| queued.insert(*pos)),
        );
    }
    fn poll_loading(&mut self, world: &mut voxy_world::World) -> Result<bool, JsValue> {
        if let Some(loading) = self.loading.as_mut() {
            let Some(chunk) = loading.try_read().map_err(error)? else {
                return Ok(false);
            };
            let find = |name: &str| {
                world
                    .registry()
                    .find(&ResourceKey::parse(name).map_err(error)?)
                    .ok_or_else(|| error("missing terrain state"))
            };
            let target = voxy_world::TerrainPalette {
                air: voxy_world::BlockStateId::AIR,
                surface: find("voxy:grass")?,
                soil: find("voxy:dirt")?,
                stone: find("voxy:stone")?,
            };
            let (palette, water) = super::browser::terrain_palette().map_err(error)?;
            let pos = chunk.pos;
            let prepared = super::voxel_terrain::PreparedTerrain::new(
                vec![chunk],
                voxy_world::WorldSeed(42),
                [
                    palette.air,
                    palette.surface,
                    palette.soil,
                    palette.stone,
                    water,
                ],
                target,
                self.states.0[7],
            )
            .map_err(error)?;
            let generated = prepared
                .generate(
                    pos,
                    voxy_world::WorldSeed(42),
                    &voxy_core::CancelToken::new(),
                )
                .map_err(error)?;
            world.insert_generated(generated).map_err(error)?;
            self.loading = None;
            self.loaded += 1;
            self.visible.insert(pos);
            self.mesh_dirty = true;
        }
        Ok(true)
    }
    pub fn poll(
        &mut self,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
        terrain: &voxy_gpu::TerrainProgram,
        world: &mut voxy_world::World,
    ) -> Result<Option<bool>, JsValue> {
        let started = self.started.get_or_insert_with(js_sys::Date::now);
        if js_sys::Date::now() - *started >= 30_000.0 {
            return Err(error("gameplay water timed out"));
        }
        if !self.poll_loading(world)? {
            return Ok(None);
        }
        if self.pending.is_none() {
            self.trim_resident_chunks(world)?;
            self.pending = Some(
                self.program
                    .begin_plan_world(
                        queue,
                        world,
                        world.registry(),
                        self.states,
                        &self.active[..self.active.len().min(WaterBudget::default().max_active)],
                        EditSource::Simulation,
                        WaterBudget::default(),
                    )
                    .map_err(error)?,
            );
        }
        let result = self
            .pending
            .as_mut()
            .ok_or_else(|| error("missing water plan"))?
            .try_plan();
        let plan = match result {
            Err(WaterSnapshotError::MissingSample(pos)) => {
                let Sample::Unloaded { chunk } = world.sample(pos) else {
                    return Err(error(
                        "water requires unavailable data; generation cannot replace it",
                    ));
                };
                if let Some(snapshot) = self.retained.get(&chunk) {
                    world.restore_chunk(snapshot).map_err(error)?;
                    self.retained.remove(&chunk);
                    self.visible.insert(chunk);
                    self.mesh_dirty = true;
                    self.pending = None;
                    return Ok(None);
                }
                if self.loaded >= 64 {
                    return Err(error("water streaming chunk budget exceeded"));
                }
                let job = terrain
                    .create_job(
                        device,
                        chunk,
                        voxy_world::WorldSeed(42),
                        &voxy_core::CancelToken::new(),
                    )
                    .map_err(error)?;
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                let dispatch = job.encode(&mut encoder).map_err(error)?;
                queue.submit([encoder.finish()]);
                self.loading = Some(dispatch.begin_read());
                self.pending = None;
                return Ok(None);
            }
            result => result.map_err(error)?,
        };
        let Some(plan) = plan else {
            return Ok(None);
        };
        self.pending = None;
        match plan {
            WaterPlan::Settled => {
                self.finish_batch(&[]);
                self.started = None;
                Ok(self.active.is_empty().then_some(false))
            }
            WaterPlan::Transaction { edit, next_active } => {
                let changed: Vec<_> = edit
                    .writes
                    .iter()
                    .map(|write| voxy_core::split_voxel(write.pos).0)
                    .collect();
                match world.commit(edit) {
                    Ok(_) => {
                        self.visible.extend(changed);
                        self.mesh_dirty = true;
                        self.finish_batch(&next_active.into_vec());
                        self.started = None;
                        Ok(Some(true))
                    }
                    Err(CommitError::RevisionConflict { .. }) => Ok(None),
                    Err(failure) => Err(error(failure)),
                }
            }
        }
    }
}

fn center_water(world: &voxy_world::World, levels: [voxy_world::BlockStateId; 8]) -> Vec<VoxelPos> {
    chunk_water(world, levels, voxy_core::ChunkPos { x: 0, y: 0, z: 0 })
}

fn chunk_water(
    world: &voxy_world::World,
    levels: [voxy_world::BlockStateId; 8],
    chunk: voxy_core::ChunkPos,
) -> Vec<VoxelPos> {
    let mut active = Vec::new();
    for y in 0..32 {
        for z in 0..32 {
            for x in 0..32 {
                let Ok(pos) = voxy_core::join_voxel(
                    chunk,
                    voxy_core::LocalPos::new(x, y, z).expect("bounded chunk coordinates"),
                ) else {
                    continue;
                };
                if matches!(world.sample(pos), Sample::Loaded(block) if levels.contains(&block)) {
                    active.push(pos);
                }
            }
        }
    }
    active
}

/// Exercises the gameplay loader against an independent CPU-generated neighbor.
pub(super) async fn validate_loading(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    terrain: &voxy_gpu::TerrainProgram,
    original: &voxy_world::World,
) -> Result<u32, JsValue> {
    use voxy_world::{BlockStateId, EditTxn, TerrainPalette, VoxelWrite, WorldSeed};
    let mut world = original.clone();
    let mut simulation = BrowserWater::new(device, &world).await?;
    let source = VoxelPos {
        x: 63,
        y: 17,
        z: 16,
    };
    let find = |name: &str| {
        original
            .registry()
            .find(&ResourceKey::parse(name).map_err(error)?)
            .ok_or_else(|| error("missing streaming fixture state"))
    };
    let stone = find("voxy:stone")?;
    world
        .commit(EditTxn {
            source: EditSource::Simulation,
            expected: vec![],
            writes: vec![
                VoxelWrite {
                    pos: source,
                    block: simulation.states.0[7],
                },
                VoxelWrite {
                    pos: VoxelPos { y: 16, ..source },
                    block: stone,
                },
                VoxelWrite {
                    pos: VoxelPos { x: 62, ..source },
                    block: BlockStateId::AIR,
                },
            ],
        })
        .map_err(error)?;
    simulation.active = vec![source];
    let mut reference = world.clone();
    let neighbor = voxy_core::ChunkPos { x: 2, y: 0, z: 0 };
    let generator = voxy_world::ProceduralTerrainGenerator::new(
        TerrainPalette {
            air: BlockStateId::AIR,
            surface: find("voxy:grass")?,
            soil: find("voxy:dirt")?,
            stone,
        },
        simulation.states.0[7],
    );
    reference
        .insert_generated(
            generator
                .generate(neighbor, WorldSeed(42), &voxy_core::CancelToken::new())
                .map_err(error)?,
        )
        .map_err(error)?;
    let expected = physics_voxel::step_water(
        &reference,
        reference.registry(),
        simulation.states,
        &[source],
        EditSource::Simulation,
        WaterBudget::default(),
    )
    .map_err(error)?;
    let WaterPlan::Transaction { edit, .. } = expected else {
        return Err(error("streaming reference unexpectedly settled"));
    };
    reference.commit(edit).map_err(error)?;
    loop {
        match simulation.poll(queue, device, terrain, &mut world)? {
            Some(true) => break,
            Some(false) => return Err(error("streaming GPU unexpectedly settled")),
            None => super::browser::yield_browser().await?,
        }
    }
    if simulation.loaded != 1 {
        return Err(error("streaming loader did not load exactly one neighbor"));
    }
    if !simulation.needs_mesh_refresh() {
        return Err(error("streamed geometry publication was not requested"));
    }
    validate_automatic_retirement(device, &world).await?;
    validate_neighbor_wake(&mut simulation, &world, neighbor)?;
    validate_batch_queue(&mut simulation)?;
    validate_retirement(device, queue, terrain, &world, neighbor).await?;
    validate_gpu_batches(device, queue, terrain, original, false).await?;
    validate_gpu_batches(device, queue, terrain, original, true).await?;
    validate_streamed_mesh(&world, &simulation.visible_chunks())?;
    simulation.mark_mesh_refreshed();
    if simulation.needs_mesh_refresh() {
        return Err(error("streamed geometry publication flag was not cleared"));
    }
    compare_streamed_chunks(&world, &reference, neighbor).await
}

async fn compare_streamed_chunks(
    world: &voxy_world::World,
    reference: &voxy_world::World,
    neighbor: voxy_core::ChunkPos,
) -> Result<u32, JsValue> {
    let mut compared = 0;
    for pos in [voxy_core::ChunkPos { x: 1, y: 0, z: 0 }, neighbor] {
        let actual = world
            .chunk(pos)
            .ok_or_else(|| error("GPU streamed chunk missing"))?;
        let expected = reference
            .chunk(pos)
            .ok_or_else(|| error("CPU streamed chunk missing"))?;
        for raw in 0..32_768_u16 {
            let index =
                voxy_core::LocalIndex::new(raw).ok_or_else(|| error("invalid fixture index"))?;
            if actual.data.blocks.get(index) != expected.data.blocks.get(index) {
                return Err(error(format!(
                    "streaming water CPU mismatch at {pos:?}/{raw}"
                )));
            }
            compared += 1;
        }
        super::browser::yield_browser().await?;
    }
    Ok(compared)
}

fn validate_streamed_mesh(
    world: &voxy_world::World,
    positions: &[voxy_core::ChunkPos],
) -> Result<(), JsValue> {
    let mut scene = voxy_runtime::BootstrapScene {
        anchor: voxy_core::ChunkPos { x: 0, y: 0, z: 0 },
        chunks: voxy_runtime::rebuild_bootstrap_chunks(world, positions, 1).map_err(error)?,
        world: world.clone(),
    };
    let mesh = super::voxel_scene::mesh(&scene).map_err(error)?;
    if !mesh
        .vertices()
        .iter()
        .any(|vertex| vertex.position[0] > 3.0)
    {
        return Err(error("streamed mesh has no translated neighbor geometry"));
    }
    scene
        .chunks
        .last_mut()
        .ok_or_else(|| error("camera fixture empty"))?
        .pos
        .x = 100;
    validate_camera_range(&scene)?;
    Ok(())
}

fn validate_camera_range(scene: &voxy_runtime::BootstrapScene) -> Result<(), JsValue> {
    use voxy_render::{SceneCamera, SceneProjection};
    for aspect in [1.0 / 3.0, 3.0] {
        let (eye, target, far) = super::browser::voxel_camera(Some(scene), aspect)?;
        if far <= 100.0 {
            return Err(error("expanded camera retained fixed far plane"));
        }
        let projection = SceneCamera {
            eye,
            target,
            up: glam::Vec3::Y,
            projection: SceneProjection::Perspective {
                vertical_fov: 55_f32.to_radians(),
                aspect,
                near: 0.1,
                far,
            },
        }
        .view_projection()
        .map_err(error)?;
        for x in [-1.0, 201.0] {
            for y in [-0.625, 1.375] {
                for z in [-1.0, 1.0] {
                    let clip = projection * glam::Vec4::new(x, y, z, 1.0);
                    let ndc = clip.truncate() / clip.w;
                    if clip.w <= 0.0
                        || ndc.x.abs() > 1.0001
                        || ndc.y.abs() > 1.0001
                        || !(0.0..=1.0001).contains(&ndc.z)
                    {
                        return Err(error("expanded world camera clips bounds"));
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_neighbor_wake(
    simulation: &mut BrowserWater,
    world: &voxy_world::World,
    neighbor: voxy_core::ChunkPos,
) -> Result<(), JsValue> {
    use voxy_world::{EditTxn, VoxelWrite};
    let mut wake_world = world.clone();
    wake_world
        .commit(EditTxn {
            source: EditSource::Player(1),
            expected: vec![],
            writes: vec![VoxelWrite {
                pos: voxy_core::join_voxel(
                    neighbor,
                    voxy_core::LocalPos::new(0, 31, 0).map_err(error)?,
                )
                .map_err(error)?,
                block: simulation.states.0[7],
            }],
        })
        .map_err(error)?;
    simulation.active.clear();
    simulation.wake(&wake_world);
    let seeded =
        voxy_core::join_voxel(neighbor, voxy_core::LocalPos::new(0, 31, 0).map_err(error)?)
            .map_err(error)?;
    if !simulation.active.contains(&seeded) {
        return Err(error("edited streamed neighbor water was not reactivated"));
    }
    Ok(())
}

fn validate_batch_queue(simulation: &mut BrowserWater) -> Result<(), JsValue> {
    let limit = WaterBudget::default().max_active;
    let positions = (0..limit + 2)
        .map(|index| {
            Ok(VoxelPos {
                x: i64::try_from(index).map_err(error)?,
                y: 100,
                z: 0,
            })
        })
        .collect::<Result<Vec<_>, JsValue>>()?;
    simulation.active.clone_from(&positions);
    simulation.finish_batch(&[positions[0], positions[limit]]);
    if simulation.active != [positions[limit], positions[limit + 1], positions[0]] {
        return Err(error(
            "water batch queue lost deferred cells, duplicated work or starved tail",
        ));
    }
    simulation.finish_batch(&[]);
    if !simulation.active.is_empty() {
        return Err(error("settled water batch retained completed cells"));
    }
    Ok(())
}

async fn validate_gpu_batches(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    terrain: &voxy_gpu::TerrainProgram,
    original: &voxy_world::World,
    dense_water: bool,
) -> Result<(), JsValue> {
    use voxy_world::{BlockStateId, EditTxn, VoxelWrite};
    let mut world = original.clone();
    let mut simulation = BrowserWater::new(device, &world).await?;
    let stone = world
        .registry()
        .find(&ResourceKey::parse("voxy:stone").map_err(error)?)
        .ok_or_else(|| error("batch fixture stone missing"))?;
    let mut active = Vec::new();
    let mut writes = Vec::new();
    for y in 0..16 {
        for z in 0..32 {
            for x in 0..32 {
                let pos = VoxelPos { x, y, z };
                active.push(pos);
                writes.push(VoxelWrite {
                    pos,
                    block: if dense_water {
                        simulation.states.0[7]
                    } else {
                        stone
                    },
                });
            }
        }
    }
    let source = VoxelPos {
        x: 16,
        y: 30,
        z: 16,
    };
    writes.push(VoxelWrite {
        pos: source,
        block: simulation.states.0[7],
    });
    writes.push(VoxelWrite {
        pos: VoxelPos { y: 29, ..source },
        block: BlockStateId::AIR,
    });
    world
        .commit(EditTxn {
            source: EditSource::Player(1),
            expected: vec![],
            writes,
        })
        .map_err(error)?;
    let mut reference = world.clone();
    let first = physics_voxel::step_water(
        &reference,
        reference.registry(),
        simulation.states,
        &active,
        EditSource::Simulation,
        WaterBudget::default(),
    )
    .map_err(error)?;
    if dense_water {
        let WaterPlan::Transaction { edit, .. } = first else {
            return Err(error("dense water batch performed no CPU transfers"));
        };
        reference.commit(edit).map_err(error)?;
    } else {
        if !matches!(first, WaterPlan::Settled) {
            return Err(error("solid batch did not settle on CPU"));
        }
        let second = physics_voxel::step_water(
            &reference,
            reference.registry(),
            simulation.states,
            &[source],
            EditSource::Simulation,
            WaterBudget::default(),
        )
        .map_err(error)?;
        let WaterPlan::Transaction { edit, .. } = second else {
            return Err(error("tail water did not transfer on CPU"));
        };
        reference.commit(edit).map_err(error)?;
    }
    active.push(source);
    simulation.active = active;
    loop {
        match simulation.poll(queue, device, terrain, &mut world)? {
            Some(true) => break,
            Some(false) => return Err(error("large GPU water queue settled before tail transfer")),
            None => super::browser::yield_browser().await?,
        }
    }
    compare_batch_center(&world, &reference)?;
    Ok(())
}

fn compare_batch_center(
    world: &voxy_world::World,
    reference: &voxy_world::World,
) -> Result<(), JsValue> {
    let chunk = voxy_core::ChunkPos { x: 0, y: 0, z: 0 };
    let actual = world
        .chunk(chunk)
        .ok_or_else(|| error("GPU batch chunk missing"))?;
    let expected = reference
        .chunk(chunk)
        .ok_or_else(|| error("CPU batch chunk missing"))?;
    for raw in 0..32768_u16 {
        let index = voxy_core::LocalIndex::new(raw).ok_or_else(|| error("batch index invalid"))?;
        if actual.data.blocks.get(index) != expected.data.blocks.get(index) {
            return Err(error(format!("GPU multi-batch water mismatch at {raw}")));
        }
    }
    Ok(())
}

async fn validate_retirement(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    terrain: &voxy_gpu::TerrainProgram,
    world: &voxy_world::World,
    neighbor: voxy_core::ChunkPos,
) -> Result<(), JsValue> {
    use voxy_world::{BlockStateId, EditTxn, VoxelWrite};
    let mut retained_world = world.clone();
    let mut simulation = BrowserWater::new(device, &retained_world).await?;
    let source = voxy_core::join_voxel(
        neighbor,
        voxy_core::LocalPos::new(16, 30, 16).map_err(error)?,
    )
    .map_err(error)?;
    retained_world
        .commit(EditTxn {
            source: EditSource::Player(1),
            expected: vec![],
            writes: vec![
                VoxelWrite {
                    pos: source,
                    block: simulation.states.0[7],
                },
                VoxelWrite {
                    pos: VoxelPos {
                        y: source.y - 1,
                        ..source
                    },
                    block: BlockStateId::AIR,
                },
            ],
        })
        .map_err(error)?;
    let mut reference = retained_world.clone();
    let expected = physics_voxel::step_water(
        &reference,
        reference.registry(),
        simulation.states,
        &[source],
        EditSource::Simulation,
        WaterBudget::default(),
    )
    .map_err(error)?;
    let WaterPlan::Transaction { edit, .. } = expected else {
        return Err(error("retained water CPU fixture did not transfer"));
    };
    reference.commit(edit).map_err(error)?;
    let old_revision = retained_world
        .chunk(neighbor)
        .ok_or_else(|| error("retirement fixture chunk missing"))?
        .revision;
    simulation.active.clear();
    simulation.protect_chunks([neighbor].into_iter().collect());
    if simulation
        .retire_chunk(&mut retained_world, neighbor)
        .is_ok()
        || retained_world.chunk(neighbor).map(|chunk| chunk.revision) != Some(old_revision)
        || !simulation.retained.is_empty()
    {
        return Err(error("actor pin failed to prevent chunk retirement"));
    }
    simulation.protect_chunks(std::collections::BTreeSet::new());
    simulation.retire_chunk(&mut retained_world, neighbor)?;
    simulation.active = vec![source];
    loop {
        match simulation.poll(queue, device, terrain, &mut retained_world)? {
            Some(true) => break,
            Some(false) => return Err(error("retained water GPU retry unexpectedly settled")),
            None => super::browser::yield_browser().await?,
        }
    }
    if simulation.loaded != 0
        || simulation.retained.contains_key(&neighbor)
        || !simulation.visible.contains(&neighbor)
    {
        return Err(error(
            "retained reload regenerated terrain or failed to republish visibility",
        ));
    }
    let actual = retained_world
        .chunk(neighbor)
        .ok_or_else(|| error("retained GPU chunk missing"))?;
    if actual.revision <= old_revision {
        return Err(error("retained reload reused a captured revision"));
    }
    let expected = reference
        .chunk(neighbor)
        .ok_or_else(|| error("retained CPU chunk missing"))?;
    for raw in 0..32768_u16 {
        let index =
            voxy_core::LocalIndex::new(raw).ok_or_else(|| error("retained index invalid"))?;
        if actual.data.blocks.get(index) != expected.data.blocks.get(index) {
            return Err(error(format!("retained GPU water mismatch at {raw}")));
        }
    }
    Ok(())
}

async fn validate_automatic_retirement(
    device: &wgpu::Device,
    original: &voxy_world::World,
) -> Result<(), JsValue> {
    let mut world = original.clone();
    let mut simulation = BrowserWater::new(device, &world).await?;
    simulation.active.clear();
    for x in 100..134 {
        let pos = voxy_core::ChunkPos { x, y: 0, z: 0 };
        world
            .insert_generated(voxy_world::GeneratedChunk {
                pos,
                data: voxy_world::ChunkData::uniform(voxy_world::BlockStateId::AIR),
            })
            .map_err(error)?;
        simulation.visible.insert(pos);
    }
    simulation.protect_chunks(simulation.visible.clone());
    simulation.trim_resident_chunks(&mut world)?;
    if simulation.visible.len() != 35 || !simulation.retained.is_empty() {
        return Err(error("residency target overrode protected chunks"));
    }
    let actor = voxy_core::ChunkPos { x: 100, y: 0, z: 0 };
    let water = voxy_core::ChunkPos { x: 101, y: 0, z: 0 };
    simulation.protect_chunks([actor].into_iter().collect());
    simulation.active.push(
        voxy_core::join_voxel(water, voxy_core::LocalPos::new(31, 0, 0).map_err(error)?)
            .map_err(error)?,
    );
    simulation.trim_resident_chunks(&mut world)?;
    if simulation.visible.len() != 32
        || simulation.retained.len() != 3
        || world.chunk(actor).is_none()
        || world.chunk(water).is_none()
        || world
            .chunk(voxy_core::ChunkPos { x: 102, y: 0, z: 0 })
            .is_none()
        || !simulation.mesh_dirty
    {
        return Err(error(
            "automatic retirement lost protected data or missed its target",
        ));
    }
    for (&pos, snapshot) in &simulation.retained {
        if world.chunk(pos).is_some() {
            return Err(error("retired chunk still resident"));
        }
        world.restore_chunk(snapshot).map_err(error)?;
        if world
            .chunk(pos)
            .ok_or_else(|| error("automatic archive restore missing"))?
            .data
            != snapshot.data
        {
            return Err(error("automatic archive changed chunk data"));
        }
    }
    Ok(())
}
