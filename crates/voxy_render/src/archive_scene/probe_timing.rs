use super::{ArchiveSceneRenderer, RefineCrossover};
use crate::{ComputeMemoryBudget, ComputeStorage};
use glam::Vec3;
use std::sync::Arc;

#[derive(Debug)]
pub(super) struct Timing {
    pub(super) query_count: u32,
    pub(super) base_count: u32,
    pub(super) phase_chunks: u32,
    pub(super) sample_chunks: u32,
    pub(super) set: wgpu::QuerySet,
    pub(super) resolve: ComputeStorage,
    pub(super) readback: ComputeStorage,
    pub(super) ready: Arc<std::sync::atomic::AtomicU8>,
    pub(super) pending: bool,
    pub(super) frame_stamp: (u64, bool, bool),
    pub(super) skip_inactive_gi: bool,
}

impl Timing {
    pub(super) fn new(
        device: &wgpu::Device,
        gi: bool,
        ssr: bool,
        phase_chunks: u32,
    ) -> Result<Option<Self>, crate::ComputeError> {
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return Ok(None);
        }
        let base_count = if gi && ssr {
            12
        } else if gi {
            10
        } else {
            6
        };
        let extended = crate::archive_gpu_timing::ray_stage_count(base_count, phase_chunks)
            .filter(|&n| n <= wgpu::QUERY_SET_MAX_QUERIES);
        let phase_chunks = if extended.is_some() { phase_chunks } else { 0 };
        let query_count = extended.unwrap_or(base_count);
        let [resolve, readback] = ComputeMemoryBudget::for_device(device).allocate_buffers([
            voxy_gpu_core::ManagedBufferDescriptor {
                label: "street timestamp resolve",
                size: (u64::from(query_count) * 8).div_ceil(256) * 256,
                contents: None,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            },
            voxy_gpu_core::ManagedBufferDescriptor {
                label: "street timestamp readback",
                size: u64::from(query_count) * 8,
                contents: None,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            },
        ])?;
        Ok(Some(Self {
            query_count,
            base_count,
            phase_chunks,
            sample_chunks: 0,
            set: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("street GPU frame timing"),
                ty: wgpu::QueryType::Timestamp,
                count: query_count,
            }),
            resolve,
            readback,
            ready: Arc::new(std::sync::atomic::AtomicU8::new(0)),
            pending: false,
            frame_stamp: (0, false, false),
            skip_inactive_gi: false,
        }))
    }

    pub(super) fn collect(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<([f64; 3], Option<[f64; 2]>, Option<f64>)> {
        match self.ready.swap(0, std::sync::atomic::Ordering::AcqRel) {
            1 => {
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("completed GPU stage counters"),
                });
                let count = crate::archive_gpu_timing::active_query_count(
                    self.base_count,
                    self.query_count,
                    self.sample_chunks,
                    self.skip_inactive_gi && !self.frame_stamp.2,
                );
                encoder.resolve_query_set(&self.set, 0..count, &self.resolve, 0);
                encoder.copy_buffer_to_buffer(
                    &self.resolve,
                    0,
                    &self.readback,
                    0,
                    u64::from(count) * 8,
                );
                queue.submit([encoder.finish()]);
                let ready = self.ready.clone();
                self.readback.slice(..u64::from(count) * 8).map_async(
                    wgpu::MapMode::Read,
                    move |result| {
                        ready.store(
                            if result.is_ok() { 2 } else { 3 },
                            std::sync::atomic::Ordering::Release,
                        );
                    },
                );
                return None;
            }
            2 => {}
            3 => {
                self.pending = false;
                return None;
            }
            _ => return None,
        }
        let count = crate::archive_gpu_timing::active_query_count(
            self.base_count,
            self.query_count,
            self.sample_chunks,
            self.skip_inactive_gi && !self.frame_stamp.2,
        );
        let Ok(bytes) = self
            .readback
            .slice(..u64::from(count) * 8)
            .get_mapped_range()
        else {
            self.readback.unmap();
            self.pending = false;
            return None;
        };
        let ticks: Vec<u64> = bytes
            .chunks_exact(8)
            .map(|chunk| u64::from_le_bytes(chunk.try_into().unwrap()))
            .collect();
        drop(bytes);
        self.readback.unmap();
        self.pending = false;
        if std::env::var_os("VOXY_STREET_GPU_TIMING_TRACE").is_some() {
            eprintln!(
                "STREET GPU COUNTERS ticks={ticks:?} period_ns={}",
                queue.get_timestamp_period()
            );
        }
        if self.sample_chunks > 0 && std::env::var_os("VOXY_STREET_GPU_TIMING_TRACE").is_some() {
            let phases = crate::archive_gpu_timing::ray_stage_durations(
                &ticks,
                self.base_count,
                self.sample_chunks,
                queue.get_timestamp_period(),
            );
            eprintln!("STREET GPU RAY PHASES project_traverse_resolve_ms={phases:?}");
        }
        crate::archive_gpu_timing::decode_with_rays(
            &ticks[..if self.skip_inactive_gi && !self.frame_stamp.2 {
                6
            } else {
                self.base_count as usize
            }],
            queue.get_timestamp_period(),
        )
    }
}

/// In-progress full-quality probe resources. Keep the job with its renderer;
/// camera/AA state is restored only after every original face is complete.
#[derive(Debug)]
#[must_use = "advance the bake on its owning renderer until complete"]
pub struct ArchiveProbeBake {
    owner: Arc<()>,
    render_size: (u32, u32),
    _face: Arc<voxy_gpu_core::ManagedTexture>,
    view: wgpu::TextureView,
    projector: crate::probe_gi::ProbeProjector,
    grid: crate::probe_gi::ProbeGrid,
    points: Vec<Vec3>,
    saved: (crate::ArchiveAntialiasing, bool, f32, Vec3),
    next_probe: usize,
    next_face: usize,
    pending_face: Option<wgpu::SubmissionIndex>,
    started: std::time::Instant,
    finished: bool,
}

impl ArchiveProbeBake {
    pub fn progress(&self) -> (usize, usize) {
        (self.next_probe, self.points.len())
    }
    pub fn elapsed_seconds(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }
}

impl ArchiveSceneRenderer {
    /// Begin a full-resolution bake owned by this renderer. Advance between
    /// application events so startup can continue servicing the window system.
    pub fn begin_irradiance_probe_bake(
        &mut self,
        min: Vec3,
        max: Vec3,
        spacing: f32,
    ) -> Result<ArchiveProbeBake, crate::ComputeError> {
        let (grid, points) = crate::probe_gi::grid(min, max, spacing)
            .ok_or_else(|| crate::ComputeError::Validation("invalid probe grid".into()))?;
        let face = ComputeMemoryBudget::for_device(&self.device).allocate_texture(
            &wgpu::TextureDescriptor {
                label: Some("irradiance probe face"),
                size: wgpu::Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        )?;
        if let Some(root) = std::env::var_os("VOXY_STREET_TEXTURE_SOURCE_CAPTURE") {
            self.capture_texture_sources(&std::path::PathBuf::from(root))?;
        }
        let view = face.create_view(&Default::default());
        let mut projector = crate::probe_gi::ProbeProjector::new(&self.device);
        if let Some(index) = std::env::var_os("VOXY_STREET_PROBE_PROJECTOR_TRACE_INDEX") {
            let index = index
                .to_str()
                .and_then(|s| s.parse::<usize>().ok())
                .ok_or_else(|| {
                    crate::ComputeError::Validation("invalid projector trace index".into())
                })?;
            projector.enable_trace(&self.device, points.len(), index)?;
        }
        let saved = (
            self.antialiasing,
            self.linear_output,
            self.vertical_fov,
            self.camera_up,
        );
        self.queue.write_buffer(
            &self.probe_grid,
            0,
            &[0; std::mem::size_of::<crate::probe_gi::ProbeGrid>()],
        );
        self.set_antialiasing(crate::ArchiveAntialiasing::Off)?;
        self.set_linear_output(true);
        if std::env::var_os("VOXY_STREET_INIT_TRACE").is_some() {
            eprintln!(
                "ARCHIVE PROBE begin probes={} faces={} resolution={}x{}",
                points.len(),
                points.len() * 6,
                self.width,
                self.height
            );
        }
        Ok(ArchiveProbeBake {
            owner: self.probe_bake_owner.clone(),
            render_size: (self.width, self.height),
            _face: face,
            view,
            projector,
            grid,
            points,
            saved,
            next_probe: 0,
            next_face: 0,
            pending_face: None,
            started: std::time::Instant::now(),
            finished: false,
        })
    }

    /// Advance at most one original cube face with a bounded GPU wait.
    /// A pending face retains its submission and is counted only after completion.
    pub fn advance_irradiance_probe_bake(
        &mut self,
        bake: &mut ArchiveProbeBake,
    ) -> Result<bool, crate::ComputeError> {
        self.advance_probe_face(bake, Some(std::time::Duration::from_millis(2)))
    }

    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    fn advance_probe_face(
        &mut self,
        bake: &mut ArchiveProbeBake,
        timeout: Option<std::time::Duration>,
    ) -> Result<bool, crate::ComputeError> {
        if !Arc::ptr_eq(&bake.owner, &self.probe_bake_owner) {
            return Err(crate::ComputeError::DeviceMismatch);
        }
        if bake.render_size != (self.width, self.height) {
            return Err(crate::ComputeError::Validation(
                "probe renderer resized during bake".into(),
            ));
        }
        if bake.finished {
            return Ok(true);
        }
        if bake.next_probe < bake.points.len() {
            let faces = [
                (Vec3::X, Vec3::Y),
                (Vec3::NEG_X, Vec3::Y),
                (Vec3::Z, Vec3::Y),
                (Vec3::NEG_Z, Vec3::Y),
                (Vec3::Y, Vec3::NEG_Z),
                (Vec3::NEG_Y, Vec3::Z),
            ];
            let (forward, up) = faces[bake.next_face];
            let point = bake.points[bake.next_probe];
            if bake.pending_face.is_none() {
                self.set_camera_projection(90_f32.to_radians(), up);
                self.previous_vp = None;
                let right = forward.cross(up).normalize();
                bake.pending_face =
                    Some(
                        self.draw_with_tail(&bake.view, point, forward, |renderer, encoder| {
                            bake.projector.encode_projection(
                                &renderer.device,
                                &renderer.queue,
                                encoder,
                                &bake.view,
                                (renderer.width, renderer.height),
                                &renderer.probe_sh,
                                bake.next_probe,
                                bake.next_face,
                                [forward, right, up],
                                1. / renderer.exposure,
                            );
                        }),
                    );
            }
            let submission = bake.pending_face.clone();
            match self.device.poll(wgpu::PollType::Wait {
                submission_index: submission,
                timeout,
            }) {
                Ok(_) => bake.pending_face = None,
                Err(wgpu::PollError::Timeout) => return Ok(false),
                Err(error) => {
                    return Err(crate::ComputeError::Validation(format!(
                        "probe completion: {error}"
                    )));
                }
            }
            if let Some(root) = std::env::var_os("VOXY_STREET_PROBE_FACE_CAPTURE") {
                let selected = if let Some(indices) =
                    std::env::var_os("VOXY_STREET_PROBE_FACE_CAPTURE_INDICES")
                {
                    let indices = indices.to_str().ok_or_else(|| {
                        crate::ComputeError::Validation("probe trace indices must be UTF-8".into())
                    })?;
                    let indices = indices
                        .split(',')
                        .map(|v| v.trim().parse::<usize>())
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| {
                            crate::ComputeError::Validation(format!("probe trace index: {e}"))
                        })?;
                    if indices.iter().any(|&v| v >= bake.points.len()) {
                        return Err(crate::ComputeError::Validation(
                            "probe trace index outside baked grid".into(),
                        ));
                    }
                    indices.contains(&bake.next_probe)
                } else {
                    bake.next_probe == 0
                };
                if selected {
                    let root = std::path::PathBuf::from(root);
                    std::fs::create_dir_all(&root).map_err(|e| {
                        crate::ComputeError::Validation(format!("probe trace directory: {e}"))
                    })?;
                    let mut raster_inputs = vec![
                        (
                            "probe-frame-background",
                            self.background.create_view(&Default::default()),
                        ),
                        (
                            "probe-shadow-depth",
                            self.shadow.create_view(&Default::default()),
                        ),
                    ];
                    if let Some(gi) = &self.compute_gi {
                        let [world, normal] = gi.world_ray_guides();
                        raster_inputs
                            .push(("probe-world-guide", world.create_view(&Default::default())));
                        raster_inputs.push((
                            "probe-normal-guide",
                            normal.create_view(&Default::default()),
                        ));
                    }
                    if let Some((gi, ssr)) = self.opaque_lighting_outputs() {
                        raster_inputs
                            .push(("probe-actual-gi", gi.create_view(&Default::default())));
                        raster_inputs
                            .push(("probe-actual-ssr", ssr.create_view(&Default::default())));
                    }
                    crate::archive_hdr_diagnostics::capture_probe_face(
                        &self.device,
                        &self.queue,
                        crate::archive_hdr_diagnostics::ProbeFaceSources {
                            uniform_inputs: self.diagnostic_lighting_material_uniforms(),
                            raster_inputs,
                            face_view: &bake.view,
                            projection: bake.projector.diagnostic_uniform(),
                            camera: &self.camera,
                            coefficients: &self.probe_sh,
                            point_bits: point.to_array().map(f32::to_bits),
                            probe_index: bake.next_probe,
                            face_index: bake.next_face,
                        },
                        &root.join(format!(
                            "probe-{:04}-face-{}",
                            bake.next_probe, bake.next_face
                        )),
                    )
                    .map_err(|e| {
                        crate::ComputeError::Validation(format!("original probe face trace: {e}"))
                    })?;
                }
            }
            bake.next_face += 1;
            if bake.next_face == 6 {
                bake.next_face = 0;
                bake.next_probe += 1;
                if std::env::var_os("VOXY_STREET_INIT_TRACE").is_some() {
                    eprintln!(
                        "ARCHIVE PROBE completed={}/{} elapsed_seconds={:.3}",
                        bake.next_probe,
                        bake.points.len(),
                        bake.started.elapsed().as_secs_f64()
                    );
                }
            }
        }
        if bake.next_probe < bake.points.len() {
            return Ok(false);
        }
        // If restoring temporal resources fails, a retry finalizes the same
        // completed coefficients instead of indexing beyond the probe list.
        self.queue
            .write_buffer(&self.probe_grid, 0, bytemuck::bytes_of(&bake.grid));
        self.set_linear_output(bake.saved.1);
        self.set_camera_projection(bake.saved.2, bake.saved.3);
        self.set_antialiasing(bake.saved.0)?;
        self.previous_vp = None;
        self.reset_temporal_history();
        if let Some(root) = std::env::var_os("VOXY_STREET_TEXTURE_SOURCE_CAPTURE_AFTER_BAKE") {
            self.capture_texture_sources(&std::path::PathBuf::from(root))?;
        }
        bake.projector.save_trace(&self.device, &self.queue)?;
        bake.finished = true;
        Ok(true)
    }

    /// Synchronous compatibility entry over the same face sequence and owner.
    pub fn bake_irradiance_probes(
        &mut self,
        min: Vec3,
        max: Vec3,
        spacing: f32,
    ) -> Result<usize, crate::ComputeError> {
        let mut bake = self.begin_irradiance_probe_bake(min, max, spacing)?;
        while !self.advance_probe_face(&mut bake, None)? {}
        Ok(bake.points.len())
    }

    /// Reads the coefficients of a completed bake back from the GPU as a `.vprobes` cache.
    /// Blocks until the copy is mapped; call it once, right after the bake reports completion.
    /// # Errors
    /// Rejects a bake owned by another renderer or one that has not finished, and reports
    /// device-loss or mapping failures.
    pub fn read_irradiance_probe_cache(
        &self,
        bake: &ArchiveProbeBake,
    ) -> Result<crate::probe_gi::BakedProbeCache, crate::ComputeError> {
        if !Arc::ptr_eq(&bake.owner, &self.probe_bake_owner) {
            return Err(crate::ComputeError::DeviceMismatch);
        }
        if !bake.finished {
            return Err(crate::ComputeError::Validation(
                "probe bake is not complete".into(),
            ));
        }
        let size = bake.points.len() as u64 * crate::probe_gi::PROBE_BYTES;
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("irradiance probe cache readback"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&self.probe_sh, 0, &staging, 0, size);
        self.queue.submit([encoder.finish()]);
        let (sender, receiver) = std::sync::mpsc::channel();
        let slice = staging.slice(..);
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        let failed = |reason: String| {
            crate::ComputeError::Validation(format!("probe cache readback: {reason}"))
        };
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| failed(e.to_string()))?;
        receiver
            .recv()
            .map_err(|e| failed(e.to_string()))?
            .map_err(|e| failed(e.to_string()))?;
        let probes: Vec<[f32; 16]> = bytemuck::pod_collect_to_vec(
            &slice
                .get_mapped_range()
                .map_err(|e| failed(e.to_string()))?,
        );
        crate::probe_gi::BakedProbeCache::new(
            bake.grid.origin,
            bake.grid.spacing,
            bake.grid.dims,
            probes,
        )
        .map_err(|e| failed(e.to_string()))
    }

    /// Uploads a cached bake instead of rendering probe faces. Returns `false`, changing nothing,
    /// when the cache was baked for a different grid than `min`/`max`/`spacing` produce here.
    /// The cache does not know the scene it was baked from: the caller decides when it is stale.
    /// # Errors
    /// Rejects an invalid probe grid request.
    pub fn restore_irradiance_probe_cache(
        &mut self,
        min: Vec3,
        max: Vec3,
        spacing: f32,
        cache: &crate::probe_gi::BakedProbeCache,
    ) -> Result<bool, crate::ComputeError> {
        let (grid, points) = crate::probe_gi::grid(min, max, spacing)
            .ok_or_else(|| crate::ComputeError::Validation("invalid probe grid".into()))?;
        if cache.origin != grid.origin
            || cache.spacing != grid.spacing
            || cache.dims != grid.dims
            || cache.probes.len() != points.len()
        {
            return Ok(false);
        }
        cache.upload_to_buffer(&self.queue, &self.probe_sh);
        self.queue
            .write_buffer(&self.probe_grid, 0, bytemuck::bytes_of(&grid));
        self.previous_vp = None;
        self.reset_temporal_history();
        Ok(true)
    }

    /// Full-precision coefficients for cache/diagnostic readback; ownership
    /// remains with this renderer and callers must wait for GPU completion.
    #[doc(hidden)]
    pub fn irradiance_probe_coefficients_buffer(&self) -> &wgpu::Buffer {
        &self.probe_sh
    }

    /// Optional completed-frame diagnostics; does not change rendering inputs.
    fn capture_texture_sources(&self, root: &std::path::Path) -> Result<(), crate::ComputeError> {
        std::fs::create_dir(root)
            .map_err(|e| crate::ComputeError::Validation(format!("texture source trace: {e}")))?;
        let mut sources: Vec<(String, &wgpu::Texture)> = self
            ._textures
            .iter()
            .enumerate()
            .map(|(index, texture)| (format!("material-texture-{index:04}"), &***texture))
            .collect();
        sources.extend([
            (
                "environment-specular".to_string(),
                self._environment.specular().output(),
            ),
            (
                "environment-diffuse".to_string(),
                self._environment.diffuse().output(),
            ),
            (
                "environment-dfg".to_string(),
                self._environment.dfg().output(),
            ),
            (
                "environment-input".to_string(),
                self._environment.specular().input(),
            ),
            ("sky".to_string(), &*self._sky),
        ]);
        let mut proof = serde_json::Map::new();
        for (name, texture) in sources {
            let chain = crate::archive_hdr_diagnostics::capture_texture_chain(
                &self.device,
                &self.queue,
                texture,
                &root.join(&name),
            )
            .map_err(|e| {
                crate::ComputeError::Validation(format!("original texture chain {name}: {e}"))
            })?;
            proof.insert(name, chain);
        }
        std::fs::write(
            root.join("sources.json"),
            serde_json::to_vec_pretty(&proof)
                .map_err(|e| crate::ComputeError::Validation(e.to_string()))?,
        )
        .map_err(|e| crate::ComputeError::Validation(e.to_string()))?;
        Ok(())
    }

    fn diagnostic_lighting_material_uniforms(&self) -> Vec<(String, &wgpu::Buffer)> {
        let mut inputs = vec![
            ("punctual-lights".to_string(), &*self._lights),
            (
                "environment-intensity".to_string(),
                &self.environment_bindings.intensity_buffer,
            ),
            ("probe-grid".to_string(), &*self.probe_grid),
        ];
        inputs.extend(
            self.materials
                .iter()
                .enumerate()
                .map(|(index, material)| (format!("material-{index:04}"), &*material._uniform)),
        );
        inputs
    }

    pub fn capture_hdr_stages(
        &self,
        resolve: &crate::SupersampleResolve,
        output: &wgpu::Texture,
        root: &std::path::Path,
        raw: bool,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let mut views = self.temporal.as_ref().map_or_else(
            Vec::new,
            super::super::archive_temporal::ArchiveTemporal::diagnostic_views,
        );
        views.push((
            "display-before-downsample",
            resolve.color.create_view(&Default::default()),
        ));
        views.push(("final-output", output.create_view(&Default::default())));
        let sources = std::env::var("VOXY_STREET_HDR_SOURCE_CAPTURE").as_deref() == Ok("1");
        if sources {
            if let Some((gi, ssr)) = self.opaque_lighting_outputs() {
                views.push(("actual-gi", gi.create_view(&Default::default())));
                views.push(("actual-ssr", ssr.create_view(&Default::default())));
            }
            if let Some(gi) = &self.compute_gi {
                let [world, normal] = gi.world_ray_guides();
                views.push(("actual-world-guide", world.create_view(&Default::default())));
                views.push((
                    "actual-normal-guide",
                    normal.create_view(&Default::default()),
                ));
            }
        }
        if std::env::var_os("VOXY_STREET_GI_RAW_CAPTURE").is_some() {
            if !self.compute_gi_active_for_frame() {
                return Err("raw GI capture requires an active compute frame".into());
            }
            if let Some((actual, reference)) = self.opaque_gi_precision_textures() {
                if !sources {
                    views.push(("actual-gi", actual.create_view(&Default::default())));
                }
                views.push(("reference-gi", reference.create_view(&Default::default())));
            }
            if !sources && let Some(gi) = &self.compute_gi {
                let [world, normal] = gi.world_ray_guides();
                views.push(("actual-world-guide", world.create_view(&Default::default())));
                views.push((
                    "actual-normal-guide",
                    normal.create_view(&Default::default()),
                ));
            }
        }
        let mut proof = crate::archive_hdr_diagnostics::capture(
            &self.device,
            &self.queue,
            views,
            self.temporal
                .as_ref()
                .map(super::super::archive_temporal::ArchiveTemporal::diagnostic_uniform),
            root,
            raw,
        )?;
        if sources {
            proof["source_query_encodes_before_capture"] =
                serde_json::json!(self.world_ray_query_encoded_frames());
            proof["irradiance_probe_coefficients"] =
                crate::archive_hdr_diagnostics::capture_storage(
                    &self.device,
                    &self.queue,
                    self.irradiance_probe_coefficients_buffer(),
                    &root.join("probe-sh.native.bin"),
                    raw,
                )?;
            proof["scene_camera_uniform"] = crate::archive_hdr_diagnostics::capture_uniform(
                &self.device,
                &self.queue,
                &self.camera,
                &root.join("scene-camera-uniform.bin"),
            )?;
            let mut lighting = serde_json::Map::new();
            for (name, buffer) in self.diagnostic_lighting_material_uniforms() {
                lighting.insert(
                    name.clone(),
                    crate::archive_hdr_diagnostics::capture_uniform(
                        &self.device,
                        &self.queue,
                        buffer,
                        &root.join(format!("{name}.bin")),
                    )?,
                );
            }
            proof["lighting_material_uniforms"] = serde_json::Value::Object(lighting);
            std::fs::write(root.join("stages.json"), serde_json::to_vec_pretty(&proof)?)?;
        }
        Ok(proof)
    }

    /// The app supplies the rendered replay ordinal before encoding its frame.
    pub fn set_benchmark_frame(&mut self, frame: u64) {
        self.benchmark_frame = frame;
    }

    pub fn refinement_crossover(&self) -> Option<RefineCrossover> {
        self.refine_crossover
    }

    pub fn crossover_candidate(&self) -> Option<&'static str> {
        self.refine_crossover.map(|_| {
            if self.gi_stage_crossover {
                "original-gi-compute"
            } else {
                "refinement-cache"
            }
        })
    }

    pub(super) fn collect_gpu_time_sample(&mut self) {
        if let Some(timing) = &mut self.timing
            && let Some((ms, gi, ssr)) = timing.collect(&self.device, &self.queue)
        {
            self.gpu_frame_stamps.push(timing.frame_stamp);
            self.gpu_times_ms.push(ms[0]);
            self.gpu_shading_times_ms.push([ms[1], ms[2]]);
            self.gpu_gi_times_ms.push(gi);
            self.gpu_ssr_times_ms.push(ssr);
        }
    }

    /// Finish delayed timestamp readback without drawing an extra frame.
    /// Intended for explicit diagnostics/benchmark completion, not interactive draws.
    pub fn collect_completed_gpu_times(&mut self) -> Result<(), crate::ComputeError> {
        for _ in 0..3 {
            self.device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: None,
                })
                .map_err(|e| {
                    crate::ComputeError::Validation(format!("completed timestamp readback: {e}"))
                })?;
            self.collect_gpu_time_sample();
            if self.timing.as_ref().is_none_or(|t| !t.pending) {
                return Ok(());
            }
        }
        Err(crate::ComputeError::Validation(
            "completed timestamp readback did not finish".into(),
        ))
    }
}

#[cfg(all(test, target_os = "macos"))]
mod bake_tests {
    use super::*;

    #[test]
    #[ignore = "requires a physical Metal GPU"]
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    fn single_submission_probe_bake_matches_separate_submissions() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let min = Vec3::ZERO;
        let max = Vec3::ONE;
        let renderer = || {
            ArchiveSceneRenderer::new(
                &device,
                &queue,
                wgpu::TextureFormat::Rgba16Float,
                16,
                16,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                &[],
                Vec3::Y,
                [1.; 4],
                (min, max),
            )
            .unwrap()
        };
        let read = |renderer: &ArchiveSceneRenderer| {
            let source = renderer.irradiance_probe_coefficients_buffer();
            let size = 8 * crate::probe_gi::PROBE_BYTES;
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut encoder = device.create_command_encoder(&Default::default());
            encoder.copy_buffer_to_buffer(source, 0, &buffer, 0, size);
            queue.submit([encoder.finish()]);
            let (sender, receiver) = std::sync::mpsc::channel();
            let slice = buffer.slice(..);
            slice.map_async(wgpu::MapMode::Read, move |result| {
                sender.send(result).unwrap();
            });
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            receiver.recv().unwrap().unwrap();
            slice.get_mapped_range().unwrap().to_vec()
        };

        let mut reference = renderer();
        let mut reference_bake = reference.begin_irradiance_probe_bake(min, max, 1.).unwrap();
        let faces = [
            (Vec3::X, Vec3::Y),
            (Vec3::NEG_X, Vec3::Y),
            (Vec3::Z, Vec3::Y),
            (Vec3::NEG_Z, Vec3::Y),
            (Vec3::Y, Vec3::NEG_Z),
            (Vec3::NEG_Y, Vec3::Z),
        ];
        loop {
            let (forward, up) = faces[reference_bake.next_face];
            let point = reference_bake.points[reference_bake.next_probe];
            reference.set_camera_projection(90_f32.to_radians(), up);
            reference.previous_vp = None;
            reference.draw(&reference_bake.view, point, forward);

            let mut encoder = device.create_command_encoder(&Default::default());
            reference_bake.projector.encode_projection(
                &device,
                &queue,
                &mut encoder,
                &reference_bake.view,
                (reference.width, reference.height),
                &reference.probe_sh,
                reference_bake.next_probe,
                reference_bake.next_face,
                [forward, forward.cross(up).normalize(), up],
                1. / reference.exposure,
            );
            reference_bake.pending_face = Some(queue.submit([encoder.finish()]));
            if reference
                .advance_probe_face(&mut reference_bake, None)
                .unwrap()
            {
                break;
            }
        }
        assert_eq!(reference_bake.progress(), (8, 8));
        let expected = read(&reference);
        assert!(expected.iter().any(|&byte| byte != 0));

        let mut bounded = renderer();
        let mut bake = bounded.begin_irradiance_probe_bake(min, max, 1.).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_mins(1);
        while !bounded.advance_irradiance_probe_bake(&mut bake).unwrap() {
            assert!(std::time::Instant::now() < deadline);
        }
        assert_eq!(bake.progress(), (8, 8));
        assert!(bake.pending_face.is_none());
        assert_eq!(read(&bounded), expected);

        // A cache read back from the bake restores identical coefficients without rendering.
        let cache = bounded.read_irradiance_probe_cache(&bake).unwrap();
        let cache =
            crate::probe_gi::BakedProbeCache::from_bytes(&cache.to_bytes().unwrap()).unwrap();
        let mut restored = renderer();
        assert!(
            !restored
                .restore_irradiance_probe_cache(min, max, 0.5, &cache)
                .unwrap()
        );
        assert!(read(&restored).iter().all(|&byte| byte == 0));
        assert!(
            restored
                .restore_irradiance_probe_cache(min, max, 1., &cache)
                .unwrap()
        );
        assert_eq!(read(&restored), expected);
    }
}
