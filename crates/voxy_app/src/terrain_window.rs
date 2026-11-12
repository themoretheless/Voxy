//! Application publication of a prepared terrain window.
use super::*;
use voxy_world::VoxelView;

#[derive(Debug)]
struct TerrainPreparation {
    generated: voxy_runtime::GeneratedTerrainWindow,
    retained: BTreeMap<ChunkPos, voxy_world::ChunkSnapshot>,
    archived: BTreeMap<ChunkPos, voxy_world::ChunkRevision>,
    meshes: BTreeMap<ChunkPos, BootstrapChunk>,
    content: TerrainContent,
    source_revisions: BTreeMap<ChunkPos, voxy_world::ChunkRevision>,
}

struct TerrainRequest {
    requested: Instant,
    center: ChunkPos,
    radius: i32,
    cancel: voxy_core::CancelToken,
    existing: BTreeMap<ChunkPos, BootstrapChunk>,
}

impl TerrainPreparation {
    fn prepare(
        source: &voxy_runtime::BootstrapTerrainSource,
        archive: &voxy_runtime::TerrainArchive,
        world: &voxy_world::World,
        request: TerrainRequest,
        water: WaterStates,
        plants: Option<&native_plants::NativePlants>,
    ) -> Result<Self, String> {
        let TerrainRequest {
            requested,
            center,
            radius,
            cancel,
            existing,
        } = request;
        let started = Instant::now();
        let retained = archive
            .load_window(world, center, radius)
            .map_err(|error| error.to_string())?;
        let loaded = Instant::now();
        let generated = source
            .generate_window(world, &retained, center, radius, &cancel)
            .map_err(|error| error.to_string())?;
        println!(
            "Voxy terrain window generated: center={center:?} queue_ms={:.3} read_ms={:.3} generate_ms={:.3}",
            started.duration_since(requested).as_secs_f64() * 1000.,
            loaded.duration_since(started).as_secs_f64() * 1000.,
            loaded.elapsed().as_secs_f64() * 1000.,
        );
        Self::derive(
            world,
            archive,
            (generated, retained),
            (center, cancel, existing),
            water,
            plants,
        )
    }

    fn derive(
        world: &World,
        archive: &voxy_runtime::TerrainArchive,
        (generated, retained): (
            voxy_runtime::GeneratedTerrainWindow,
            BTreeMap<ChunkPos, voxy_world::ChunkSnapshot>,
        ),
        (center, cancel, existing): (
            ChunkPos,
            voxy_core::CancelToken,
            BTreeMap<ChunkPos, BootstrapChunk>,
        ),
        water: WaterStates,
        plants: Option<&native_plants::NativePlants>,
    ) -> Result<Self, String> {
        let generated_at = Instant::now();
        let prepared = generated
            .derive_reusing_cancellable(world, &retained, 0, &existing, &cancel)
            .map_err(|error| error.to_string())?;
        if cancel.is_cancelled() {
            return Err("terrain preparation cancelled".into());
        }
        let derived = Instant::now();
        archive
            .save_evictions(&prepared.world, &prepared.evicted)
            .map_err(|error| error.to_string())?;
        let archived = prepared
            .evicted
            .iter()
            .map(|chunk| (chunk.pos, chunk.revision))
            .collect();
        let scene = BootstrapScene {
            anchor: center,
            world: prepared.world,
            chunks: prepared.chunks,
        };
        let archived_at = Instant::now();
        let content = TerrainContent::prepare(&scene, water, plants)?;
        println!(
            "Voxy terrain window prepared: center={center:?} reused={} mesh_ms={:.3} archive_ms={:.3} content_ms={:.3}",
            prepared.reused_chunks,
            derived.duration_since(generated_at).as_secs_f64() * 1000.,
            archived_at.duration_since(derived).as_secs_f64() * 1000.,
            archived_at.elapsed().as_secs_f64() * 1000.,
        );
        let meshes = scene
            .chunks
            .into_iter()
            .map(|chunk| (chunk.pos, chunk))
            .collect();
        Ok(Self {
            generated,
            retained,
            archived,
            meshes,
            content,
            source_revisions: world
                .resident_chunk_positions()
                .filter_map(|pos| world.chunk(pos).map(|chunk| (pos, chunk.revision)))
                .collect(),
        })
    }
}

#[derive(Debug)]
pub(super) struct PendingTerrain {
    requested: Instant,
    center: ChunkPos,
    cancel: voxy_core::CancelToken,
    refreshed: bool,
    worker: std::thread::JoinHandle<Result<TerrainPreparation, String>>,
}

#[derive(Debug)]
struct TerrainContent {
    water: Vec<voxy_core::VoxelPos>,
    plants: Option<native_plants::NativePlants>,
    vegetation: Option<voxy_render::NativeVegetationWindow>,
}

impl TerrainContent {
    fn prepare(
        scene: &BootstrapScene,
        water: WaterStates,
        plants: Option<&native_plants::NativePlants>,
    ) -> Result<Self, String> {
        let water = terrain_water_cells(scene, water)?;
        let (plants, vegetation) = if let Some(plants) = plants {
            let (plants, window) = plants
                .prepare_window(&scene.world, &scene.chunks)
                .map_err(|error| error.to_string())?;
            (Some(plants), Some(window))
        } else {
            (None, None)
        };
        Ok(Self {
            water,
            plants,
            vegetation,
        })
    }
}

impl VoxyApp {
    pub(super) fn follow_terrain_window(&mut self) -> Result<(), String> {
        let (Some(source), Some(world), Some(character), Some(water)) = (
            &self.terrain_source,
            &self.world,
            self.character,
            self.water_states,
        ) else {
            return Ok(());
        };
        let center = self.terrain_center_for(character);
        if let Some(pending) = &self.pending_terrain {
            if center != pending.center {
                pending.cancel.cancel();
            }
            if !pending.worker.is_finished() {
                return Ok(());
            }
        } else if center == self.terrain_center {
            return Ok(());
        } else {
            let requested = Instant::now();
            let source = Arc::clone(source);
            let world = world.clone();
            if self.terrain_archive.is_none() {
                self.terrain_archive = Some(Arc::new(
                    voxy_runtime::TerrainArchive::new().map_err(|error| error.to_string())?,
                ));
            }
            let archive = Arc::clone(self.terrain_archive.as_ref().expect("terrain archive"));
            let cancel = voxy_core::CancelToken::new();
            let request = TerrainRequest {
                requested,
                center,
                radius: self.terrain_radius,
                cancel: cancel.clone(),
                existing: self.terrain_meshes_for(center),
            };
            let plants = self.native_plants.clone();
            let worker = std::thread::Builder::new()
                .name("voxy-terrain-window".into())
                .spawn(move || {
                    TerrainPreparation::prepare(
                        &source,
                        &archive,
                        &world,
                        request,
                        water,
                        plants.as_ref(),
                    )
                })
                .map_err(|error| error.to_string())?;
            self.pending_terrain = Some(PendingTerrain {
                requested,
                center,
                cancel,
                refreshed: false,
                worker,
            });
            return Ok(());
        }

        let pending = self
            .pending_terrain
            .take()
            .expect("finished terrain preparation");
        self.complete_terrain_window(water, center, pending)
    }

    pub(super) fn terrain_center_for(&self, character: CharacterState) -> ChunkPos {
        let (anchor, _) = voxy_core::split_voxel(character.body.anchor);
        ChunkPos {
            y: self.terrain_center.y,
            ..anchor
        }
    }

    fn complete_terrain_window(
        &mut self,
        water: WaterStates,
        center: ChunkPos,
        pending: PendingTerrain,
    ) -> Result<(), String> {
        let background = pending
            .worker
            .join()
            .map_err(|_| "terrain worker panicked")?;
        if pending.cancel.is_cancelled() {
            return Ok(());
        }
        let world = self.world.as_ref().ok_or("terrain world unavailable")?;
        let epoch = self
            .derivation_epoch
            .checked_add(1)
            .ok_or("terrain epoch exhausted")?;
        let derivation_started = Instant::now();
        let background = background?;
        if !pending.refreshed
            && background.source_revisions.iter().any(|(&pos, &revision)| {
                world
                    .chunk(pos)
                    .is_none_or(|chunk| chunk.revision != revision)
            })
        {
            self.pending_terrain = Some(self.refresh_terrain_window(
                world,
                water,
                pending.requested,
                pending.cancel,
                center,
                background,
            )?);
            return Ok(());
        }
        let prepared = background
            .generated
            .derive_reusing(world, &background.retained, epoch, &background.meshes)
            .map_err(|error| error.to_string())?;
        self.publish_terrain_window(
            center,
            epoch,
            prepared,
            derivation_started.elapsed(),
            background.content,
            &background.archived,
        )?;
        println!(
            "Voxy terrain window latency: center={center:?} elapsed_ms={:.3}",
            pending.requested.elapsed().as_secs_f64() * 1000.
        );
        Ok(())
    }

    fn refresh_terrain_window(
        &self,
        world: &World,
        water: WaterStates,
        requested: Instant,
        cancel: voxy_core::CancelToken,
        center: ChunkPos,
        background: TerrainPreparation,
    ) -> Result<PendingTerrain, String> {
        let world = world.clone();
        let archive = Arc::clone(self.terrain_archive.as_ref().expect("terrain archive"));
        let plants = self.native_plants.clone();
        let worker_cancel = cancel.clone();
        println!("Voxy terrain window refreshing: center={center:?}");
        let worker = std::thread::Builder::new()
            .name("voxy-terrain-window".into())
            .spawn(move || {
                TerrainPreparation::derive(
                    &world,
                    &archive,
                    (background.generated, background.retained),
                    (center, worker_cancel, background.meshes),
                    water,
                    plants.as_ref(),
                )
            })
            .map_err(|error| error.to_string())?;
        Ok(PendingTerrain {
            requested,
            center,
            cancel,
            refreshed: true,
            worker,
        })
    }

    fn terrain_meshes_for(&self, center: ChunkPos) -> BTreeMap<ChunkPos, BootstrapChunk> {
        let radius = self.terrain_radius.unsigned_abs().into();
        self.derived_chunks
            .iter()
            .filter(|(pos, _)| {
                pos.y == center.y
                    && pos.x.abs_diff(center.x) <= radius
                    && pos.z.abs_diff(center.z) <= radius
            })
            .map(|(&pos, chunk)| (pos, chunk.clone()))
            .collect()
    }

    fn publish_terrain_window(
        &mut self,
        center: ChunkPos,
        epoch: u64,
        prepared: voxy_runtime::PreparedTerrainWindow,
        derivation: Duration,
        background_content: TerrainContent,
        archived: &BTreeMap<ChunkPos, voxy_world::ChunkRevision>,
    ) -> Result<(), String> {
        let renderer = self
            .renderer
            .as_mut()
            .ok_or("terrain renderer unavailable")?;
        let water = self
            .water_states
            .ok_or("terrain water states unavailable")?;
        let scene = BootstrapScene {
            anchor: center,
            world: prepared.world,
            chunks: prepared.chunks,
        };
        let content_started = Instant::now();
        // Reused mesh stamps cover the center, support and vertical trunk inputs.
        let content_reused = prepared.reused_chunks == scene.chunks.len();
        let content = if content_reused {
            background_content
        } else {
            TerrainContent::prepare(&scene, water, self.native_plants.as_ref())?
        };
        let content_time = content_started.elapsed();
        let uploads = scene
            .chunks
            .iter()
            .map(|chunk| (chunk.pos, &chunk.mesh, &chunk.light))
            .collect::<Vec<_>>();
        // Edits made during preparation may supersede the staged snapshot.
        let newer_evictions = prepared
            .evicted
            .iter()
            .filter(|chunk| archived.get(&chunk.pos) != Some(&chunk.revision))
            .cloned()
            .collect::<Vec<_>>();
        self.terrain_archive
            .as_ref()
            .ok_or("terrain archive unavailable")?
            .save_evictions(&scene.world, &newer_evictions)
            .map_err(|error| error.to_string())?;
        let upload_started = Instant::now();
        renderer
            .upload_lit_window(&uploads, self.camera_anchor, content.vegetation)
            .map_err(|error| error.to_string())?;
        let upload_time = upload_started.elapsed();

        self.native_plants = content.plants;
        self.archived_terrain += prepared.evicted.len();
        self.archived_terrain -= prepared.restored.len();
        self.resident_positions = scene.chunks.iter().map(|chunk| chunk.pos).collect();
        self.derived_chunks = scene
            .chunks
            .into_iter()
            .map(|chunk| (chunk.pos, chunk))
            .collect();
        self.world = Some(scene.world);
        self.water_active = content.water;
        self.derivation_epoch = epoch;
        self.rebuild_pending = None;
        self.dirty_chunks.clear();
        self.terrain_center = center;
        if self.profile.is_some() {
            self.profile_full_publications += 1;
            self.profile_terrain_transitions += 1;
        }
        println!(
            "Voxy terrain window published: center={center:?} visible={} retained={} reused={} derive_ms={:.3} content_reused={content_reused} content_ms={:.3} upload_ms={:.3}",
            self.derived_chunks.len(),
            self.archived_terrain,
            prepared.reused_chunks,
            derivation.as_secs_f64() * 1000.,
            content_time.as_secs_f64() * 1000.,
            upload_time.as_secs_f64() * 1000.,
        );
        Ok(())
    }
}
