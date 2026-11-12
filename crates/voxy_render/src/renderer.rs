use bytemuck::Zeroable;
use glam::{Mat4, Vec3};
use std::collections::BTreeMap;
use voxy_core::ChunkPos;
use voxy_lighting::LightVolume;
use voxy_mesher::ChunkMesh;
#[cfg(test)]
use voxy_mesher::{FaceDir, Quad, QuadDiagonal, RenderLayer};
use wgpu::util::DeviceExt;

#[cfg(test)]
use crate::MaterialLayer;
use crate::skinned::{GpuSkinnedMesh, create_skin_layout, create_skinned_pipeline, upload_skinned};
use crate::{BodycamPostProcess, BodycamRig, MaterialPack, MaterialSet};
use crate::{SkinnedMesh, SkinnedUploadError};
#[cfg(test)]
use std::sync::Arc;
mod bodycam_capture;
mod camera_math;
#[path = "resident_frame_timing.rs"]
mod resident_frame_timing;
mod skinned_lod;

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
use camera_math::PhotorealCameraUniform;
#[cfg(test)]
use camera_math::{CameraUniform, chunk_intersects_frustum};
use camera_math::{
    ChunkMeta, SharedCameraBindings, camera_half_extents, camera_uniform, camera_view_projection,
    pack_light_words, photoreal_camera_uniform, relative_chunk_origin,
};
use pipelines::ResidentChunkSlot;
pub use pipelines::{CameraView, GpuQuad, RenderOutcome, SkinnedMotionOutput, SurfaceState};
pub use voxy_gpu_core::GpuIndirectMesh;

#[derive(Debug)]
pub struct Renderer {
    frame_timing: Option<resident_frame_timing::Timing>,
    profile_pose: bool,
    pose_timing: Option<(crate::RenderPoseProfile, std::time::Duration)>,
    profile_acquire: std::time::Duration,
    profile_gpu: [Option<f64>; 4],
    surface: wgpu::Surface<'static>,
    adapter: wgpu::Adapter,
    readback_pool: crate::ComputeReadbackPool,
    compute_memory_budget: crate::ComputeMemoryBudget,
    device: wgpu::Device,
    device_failure: std::sync::Arc<std::sync::OnceLock<String>>,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    state: SurfaceState,
    clear_color: wgpu::Color,
    pipeline: pipelines::VoxelLayerPipelines,
    indirect_pipeline: wgpu::RenderPipeline,
    indirect_mesh: Option<GpuIndirectMesh>,
    skinned_pipeline: wgpu::RenderPipeline,
    skin_layout: wgpu::BindGroupLayout,
    skinned: Option<GpuSkinnedMesh>,
    skinned_lod: Option<crate::skinned_lod_gpu::GpuSkinnedLod>,
    skinned_motion: Option<crate::skinned_motion::ResidentSkinnedMotion>,
    skinned_camera_history: crate::MotionHistory,
    skinned_motion_enabled: bool,
    skinned_motion_output: Option<crate::RasterMotionPass>,
    scene_depth_motion: Option<crate::DepthMotionPass>,
    skinned_motion_metadata: Option<(u64, bool)>,
    resident_presented_frames: u64,
    camera_layout: wgpu::BindGroupLayout,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    shared_camera: Option<SharedCameraBindings>,
    camera: CameraView,
    chunk_meta_buffer: wgpu::Buffer,
    chunk_meta_capacity: usize,
    light_buffer: wgpu::Buffer,
    light_words_capacity: usize,
    material_layout: wgpu::BindGroupLayout,
    material_pack: MaterialPack,
    material_set: MaterialSet,
    material_bind_group: wgpu::BindGroup,
    quad_buffer: wgpu::Buffer,
    quad_capacity: usize,
    quad_count: u32,
    chunk_upload_bytes: usize,
    chunk_draws: Vec<pipelines::ChunkLayerDraws>,
    resident_chunk_slots: BTreeMap<ChunkPos, ResidentChunkSlot>,
    resident_camera_anchor: ChunkPos,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    native_vegetation: Option<crate::NativeVegetation>,
    bodycam_enabled: bool,
    bodycam_rig: BodycamRig,
    bodycam_flashlight_pos: Vec3,
    bodycam_flashlight_dir: Vec3,
    bodycam_subject_spheres: [[f32; 4]; 12],
    bodycam_wetness: f32,
    photoreal_camera_layout: wgpu::BindGroupLayout,
    photoreal_camera_buffer: wgpu::Buffer,
    photoreal_camera_bind_group: wgpu::BindGroup,
    shared_photoreal_camera: Option<SharedCameraBindings>,
    photoreal_pipeline: pipelines::VoxelLayerPipelines,
    photoreal_indirect_pipeline: wgpu::RenderPipeline,
    photoreal_skinned_pipeline: wgpu::RenderPipeline,
    photoreal_fibre_pipeline: wgpu::RenderPipeline,
    bodycam_post: BodycamPostProcess,
}

impl Renderer {
    /// First driver-reported device-loss diagnostic, retained for this renderer.
    #[must_use]
    pub fn device_failure(&self) -> Option<&str> {
        self.device_failure.get().map(String::as_str)
    }
    fn check_device(&self) -> Result<(), RendererError> {
        match self.device_failure() {
            Some(message) => Err(RendererError::DeviceLost(message.to_owned())),
            None => Ok(()),
        }
    }
    /// Creates a GPU instance, owned surface, adapter, and device.
    ///
    /// # Errors
    ///
    /// Returns a typed initialization error when surface, adapter, device, or capabilities fail.
    pub async fn new<T>(target: T, width: u32, height: u32) -> Result<Self, RendererError>
    where
        T: Into<wgpu::SurfaceTarget<'static>>,
    {
        Self::new_with_options(target, width, height, crate::GraphicsOptions::default()).await
    }

    /// Creates a renderer with an explicit backend policy.
    ///
    /// # Errors
    /// Returns an initialization error if the selected backend or surface is unavailable.
    pub async fn new_with_options<T>(
        target: T,
        width: u32,
        height: u32,
        options: crate::GraphicsOptions,
    ) -> Result<Self, RendererError>
    where
        T: Into<wgpu::SurfaceTarget<'static>>,
    {
        let instance = options.create_instance();
        Self::new_with_instance(target, width, height, options, instance).await
    }

    /// Creates a voxel renderer using the caller's platform/display instance.
    /// Instance backend policy must match `options`; its display must match target.
    /// # Errors
    /// Reports unavailable surfaces, adapters, devices and renderer capabilities.
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub async fn new_with_instance<T>(
        target: T,
        width: u32,
        height: u32,
        options: crate::GraphicsOptions,
        instance: wgpu::Instance,
    ) -> Result<Self, RendererError>
    where
        T: Into<wgpu::SurfaceTarget<'static>>,
    {
        let surface = instance
            .create_surface(target)
            .map_err(RendererError::CreateSurface)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: options.power_preference,
                force_fallback_adapter: options.force_fallback_adapter,
                compatible_surface: Some(&surface),
                apply_limit_buckets: false,
            })
            .await
            .map_err(RendererError::RequestAdapter)?;
        let mut required_features = wgpu::Features::empty();
        if adapter
            .features()
            .contains(wgpu::Features::MAPPABLE_PRIMARY_BUFFERS)
        {
            required_features |= wgpu::Features::MAPPABLE_PRIMARY_BUFFERS;
        }
        if options.profile_timestamps {
            required_features |= adapter.features() & wgpu::Features::TIMESTAMP_QUERY;
        }
        // Ray query is an experimental wgpu feature: requesting it is refused unless the
        // device also opts into experimental features, which needs `unsafe` and is not
        // something this crate does. Requesting it here made device creation fail on
        // every adapter that advertises it (llvmpipe and current desktop GPUs), so the
        // renderer only reports the capability through `has_ray_query` when a caller
        // created the device with the opt-in; the ray probe crate does that itself.
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Voxy device"),
                required_features,
                ..Default::default()
            })
            .await
            .map_err(RendererError::RequestDevice)?;
        let device_failure = std::sync::Arc::new(std::sync::OnceLock::new());
        let failure_callback = std::sync::Arc::clone(&device_failure);
        device.set_device_lost_callback(move |reason, message| {
            let _ = failure_callback.set(format!("{reason:?}: {message}"));
        });
        let mut config = surface
            .get_default_config(&adapter, width.max(1), height.max(1))
            .ok_or(RendererError::UnsupportedSurface)?;
        if let Some(mode) = options.present.mode()
            && surface
                .get_capabilities(&adapter)
                .present_modes
                .contains(&mode)
        {
            config.present_mode = mode;
        }
        // This device is still private to initialization, so error scopes cannot
        // capture another caller's resource operations. Unsupported shader/limit
        // combinations and allocations must return errors rather than panic.
        let allocation_scope = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let validation_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let state = if width == 0 || height == 0 {
            SurfaceState::Suspended
        } else {
            surface.configure(&device, &config);
            SurfaceState::Active
        };
        let camera_layout = create_camera_layout(&device);
        let material_layout = create_material_layout(&device);
        let pipeline = create_pipeline(&device, config.format, &camera_layout, &material_layout);
        let indirect_pipeline =
            create_indirect_pipeline(&device, config.format, &camera_layout, &material_layout);
        let skin_layout = create_skin_layout(&device);
        let skinned_pipeline = create_skinned_pipeline(
            &device,
            config.format,
            &camera_layout,
            &material_layout,
            &skin_layout,
        );
        let photoreal_camera_layout = create_photoreal_camera_layout(&device);
        let (
            photoreal_pipeline,
            photoreal_indirect_pipeline,
            photoreal_skinned_pipeline,
            photoreal_fibre_pipeline,
        ) = create_photoreal_pipelines(
            &device,
            wgpu::TextureFormat::Rgba16Float,
            &photoreal_camera_layout,
            &material_layout,
            &skin_layout,
        );
        let bodycam_post =
            BodycamPostProcess::new(&device, config.format, config.width, config.height);
        let bodycam_rig = BodycamRig::default();
        let bodycam_flashlight_pos = Vec3::new(52.0, 38.0, 52.0);
        let bodycam_flashlight_dir =
            (Vec3::new(16.0, 5.0, 16.0) - bodycam_flashlight_pos).normalize_or_zero();
        let bodycam_wetness = 0.65;
        let camera = CameraView::default();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Voxy camera uniform"),
            contents: bytemuck::bytes_of(&camera_uniform(config.width, config.height, camera)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let photoreal_camera_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Voxy photoreal camera uniform"),
                contents: bytemuck::bytes_of(&photoreal_camera_uniform(
                    config.width,
                    config.height,
                    camera,
                    &bodycam_rig,
                    bodycam_flashlight_pos,
                    bodycam_flashlight_dir,
                    bodycam_wetness,
                )),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let chunk_meta_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Voxy initial chunk metadata"),
            contents: bytemuck::bytes_of(&ChunkMeta::zeroed()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Voxy initial light data"),
            contents: bytemuck::bytes_of(&u32::from_le_bytes([0xf0; 4])),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group = create_camera_bind_group(
            &device,
            &camera_layout,
            &camera_buffer,
            &chunk_meta_buffer,
            &light_buffer,
        );
        let photoreal_camera_bind_group = create_camera_bind_group(
            &device,
            &photoreal_camera_layout,
            &photoreal_camera_buffer,
            &chunk_meta_buffer,
            &light_buffer,
        );
        let shared_camera = SharedCameraBindings::new(
            &device,
            &camera_layout,
            &camera_buffer,
            &chunk_meta_buffer,
            &light_buffer,
        );
        let shared_photoreal_camera = SharedCameraBindings::new(
            &device,
            &photoreal_camera_layout,
            &photoreal_camera_buffer,
            &chunk_meta_buffer,
            &light_buffer,
        );
        let material_pack = default_material_pack();
        let material_set = MaterialSet::upload(&device, &queue, &material_pack);
        let material_bind_group =
            create_material_bind_group(&device, &material_layout, &material_set);
        let quad_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Voxy empty quad buffer"),
            contents: bytemuck::bytes_of(&GpuQuad::zeroed()),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let (depth_texture, depth_view) = create_depth(&device, config.width, config.height);
        let validation_error = validation_scope.pop().await;
        let allocation_error = allocation_scope.pop().await;
        if let Some(error) = validation_error.or(allocation_error) {
            return Err(RendererError::Initialization(error));
        }
        let frame_timing = options
            .profile_timestamps
            .then(|| resident_frame_timing::Timing::new(&device))
            .flatten();
        Ok(Self {
            frame_timing,
            profile_pose: options.profile_timestamps,
            pose_timing: None,
            profile_acquire: std::time::Duration::ZERO,
            profile_gpu: [None; 4],
            surface,
            adapter,
            readback_pool: crate::ComputeReadbackPool::for_device(&device),
            compute_memory_budget: crate::ComputeMemoryBudget::for_device(&device),
            device,
            device_failure,
            queue,
            config,
            state,
            clear_color: wgpu::Color {
                r: 0.78,
                g: 0.78,
                b: 0.76,
                a: 1.0,
            },
            pipeline,
            indirect_pipeline,
            indirect_mesh: None,
            skinned_pipeline,
            skin_layout,
            skinned: None,
            skinned_lod: None,
            skinned_motion: None,
            skinned_camera_history: crate::MotionHistory::default(),
            skinned_motion_enabled: false,
            skinned_motion_output: None,
            scene_depth_motion: None,
            skinned_motion_metadata: None,
            resident_presented_frames: 0,
            camera_layout,
            camera_buffer,
            camera_bind_group,
            shared_camera,
            camera,
            chunk_meta_buffer,
            chunk_meta_capacity: 1,
            light_buffer,
            light_words_capacity: 1,
            material_layout,
            material_pack,
            material_set,
            material_bind_group,
            quad_buffer,
            quad_capacity: 1,
            quad_count: 0,
            chunk_upload_bytes: 0,
            chunk_draws: Vec::new(),
            resident_chunk_slots: BTreeMap::new(),
            resident_camera_anchor: ChunkPos { x: 0, y: 0, z: 0 },
            depth_texture,
            depth_view,
            native_vegetation: None,
            bodycam_enabled: false,
            bodycam_rig,
            bodycam_flashlight_pos,
            bodycam_flashlight_dir,
            bodycam_subject_spheres: [[0.0; 4]; 12],
            bodycam_wetness,
            photoreal_camera_layout,
            photoreal_camera_buffer,
            photoreal_camera_bind_group,
            shared_photoreal_camera,
            photoreal_pipeline,
            photoreal_indirect_pipeline,
            photoreal_skinned_pipeline,
            photoreal_fibre_pipeline,
            bodycam_post,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if self.device_failure().is_some() {
            return;
        }
        self.reset_skinned_motion();
        if width == 0 || height == 0 {
            self.state = SurfaceState::Suspended;
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        if self.shared_camera.is_none() {
            self.queue.write_buffer(
                &self.camera_buffer,
                0,
                bytemuck::bytes_of(&camera_uniform(width, height, self.camera)),
            );
        }
        if self.shared_photoreal_camera.is_none() {
            self.queue.write_buffer(
                &self.photoreal_camera_buffer,
                0,
                bytemuck::bytes_of(&photoreal_camera_uniform(
                    width,
                    height,
                    self.camera,
                    &self.bodycam_rig,
                    self.bodycam_flashlight_pos,
                    self.bodycam_flashlight_dir,
                    self.bodycam_wetness,
                )),
            );
        }
        self.bodycam_post
            .resize(&self.device, &self.queue, width, height);
        (self.depth_texture, self.depth_view) = create_depth(&self.device, width, height);
        self.state = SurfaceState::Active;
    }

    /// Replaces the currently resident quad stream.
    ///
    /// # Errors
    ///
    /// Rejects a mesh whose quad count does not fit the draw-instance contract.
    pub fn upload_mesh(&mut self, mesh: &ChunkMesh) -> Result<(), RendererError> {
        let origin = ChunkPos { x: 0, y: 0, z: 0 };
        self.upload_chunks(&[(origin, mesh)], origin)
    }

    /// Atomically replaces resident geometry and camera-relative chunk metadata.
    ///
    /// # Errors
    ///
    /// Rejects excessive chunk/quad counts and origins outside the camera-relative `i32` range.
    pub fn upload_chunks(
        &mut self,
        chunks: &[(ChunkPos, &ChunkMesh)],
        camera_anchor: ChunkPos,
    ) -> Result<(), RendererError> {
        self.check_device()?;
        let chunks = chunks
            .iter()
            .map(|&(pos, mesh)| (pos, mesh, None))
            .collect::<Vec<_>>();
        self.upload_chunk_data(&chunks, camera_anchor)
    }

    /// Uploads geometry together with its packed sky/block-light volume.
    ///
    /// # Errors
    ///
    /// Rejects excessive chunk/quad/light counts and camera-relative origin overflow.
    pub fn upload_lit_chunks(
        &mut self,
        chunks: &[(ChunkPos, &ChunkMesh, &LightVolume)],
        camera_anchor: ChunkPos,
    ) -> Result<(), RendererError> {
        self.check_device()?;
        let chunks = chunks
            .iter()
            .map(|&(pos, mesh, light)| (pos, mesh, Some(light.bytes())))
            .collect::<Vec<_>>();
        self.upload_chunk_data(&chunks, camera_anchor)
    }

    /// Upload a terrain window and publish its prepared vegetation placements together.
    ///
    /// # Errors
    /// Returns allocation, validation, or terrain upload errors before replacing the placements.
    pub fn upload_lit_window(
        &mut self,
        chunks: &[(ChunkPos, &ChunkMesh, &LightVolume)],
        camera_anchor: ChunkPos,
        vegetation: Option<crate::NativeVegetationWindow>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let prepared = vegetation
            .map(|window| {
                self.native_vegetation
                    .as_ref()
                    .ok_or_else(|| {
                        crate::ComputeError::Validation("native vegetation unavailable".into())
                    })?
                    .prepare_window(&self.device, window)
            })
            .transpose()?;
        self.upload_lit_chunks(chunks, camera_anchor)?;
        if let Some(prepared) = prepared {
            self.native_vegetation
                .as_mut()
                .expect("prepared native vegetation")
                .publish_window(prepared);
        }
        Ok(())
    }

    /// Bytes submitted by the last successful chunk upload, including chunk metadata.
    pub fn chunk_upload_bytes(&self) -> usize {
        self.chunk_upload_bytes
    }

    /// Allocated size of the resident quad, light and chunk metadata buffers.
    pub fn chunk_buffer_bytes(&self) -> u64 {
        self.quad_buffer.size() + self.light_buffer.size() + self.chunk_meta_buffer.size()
    }

    /// Updates resident lit chunks when every replacement fits its existing slot.
    /// Returns false before writing anything when a full layout replacement is needed.
    ///
    /// # Errors
    ///
    /// Rejects device failure or invalid light data.
    pub fn try_update_lit_chunks(
        &mut self,
        chunks: &[(ChunkPos, &ChunkMesh, &LightVolume)],
        camera_anchor: ChunkPos,
    ) -> Result<bool, RendererError> {
        self.check_device()?;
        if camera_anchor != self.resident_camera_anchor
            || self.resident_chunk_slots.len() != self.chunk_draws.len()
        {
            return Ok(false);
        }
        let mut updates = BTreeMap::new();
        for &(pos, mesh, light) in chunks {
            let Some(&slot) = self.resident_chunk_slots.get(&pos) else {
                return Ok(false);
            };
            let Some(draw) = slot.draw_ranges(mesh, self.chunk_draws[slot.index].origin) else {
                return Ok(false);
            };
            let mut light_words = Vec::new();
            pack_light_words(Some(light.bytes()), &mut light_words)?;
            let gpu_slot = u16::try_from(slot.index)
                .map_err(|_| RendererError::TooManyChunks(self.chunk_draws.len()))?;
            let quads: Vec<_> = mesh
                .opaque
                .iter()
                .chain(&mesh.cutout)
                .chain(&mesh.translucent)
                .map(|&quad| GpuQuad::from(quad).with_chunk_slot(gpu_slot))
                .collect();
            updates.insert(slot.index, (slot, quads, light_words, draw));
        }
        self.chunk_upload_bytes = 0;
        for (index, (slot, quads, light, draw)) in updates {
            self.chunk_upload_bytes +=
                size_of_val(quads.as_slice()) + size_of_val(light.as_slice());
            slot.write_update(
                &self.queue,
                &self.quad_buffer,
                &self.light_buffer,
                &quads,
                &light,
            );
            self.quad_count = self.quad_count
                - (self.chunk_draws[index].translucent.end - slot.start)
                + (draw.translucent.end - slot.start);
            self.chunk_draws[index] = draw;
        }
        Ok(true)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    fn upload_chunk_data(
        &mut self,
        chunks: &[(ChunkPos, &ChunkMesh, Option<&[u8]>)],
        camera_anchor: ChunkPos,
    ) -> Result<(), RendererError> {
        self.check_device()?;
        if chunks.len() > 1024 {
            return Err(RendererError::TooManyChunks(chunks.len()));
        }
        let quad_count: usize = chunks.iter().map(|(_, mesh, _)| mesh.quad_count()).sum();
        let quad_count = u32::try_from(quad_count).map_err(|_| RendererError::MeshTooLarge)?;
        let reserve = |mesh: &ChunkMesh, light: Option<&[u8]>| {
            if light.is_some() {
                (mesh.quad_count() / 4).clamp(16, 64)
            } else {
                0
            }
        };
        let extra: usize = chunks
            .iter()
            .map(|(_, mesh, light)| reserve(mesh, *light))
            .sum();
        let packed_count = quad_count as usize;
        // Give editable chunks room to grow only within an allocation size
        // that the packed mesh already requires (or has already allocated).
        let allocation = self
            .quad_capacity
            .max(packed_count.max(1).next_power_of_two());
        let padded =
            packed_count + extra <= allocation && u32::try_from(packed_count + extra).is_ok();
        let mut quads = Vec::with_capacity(if padded {
            packed_count + extra
        } else {
            packed_count
        });
        let mut metadata = Vec::with_capacity(chunks.len().max(1));
        let mut light_words =
            Vec::with_capacity((chunks.len() * voxy_core::CHUNK_VOLUME / 4).max(1));
        let mut chunk_draws = Vec::with_capacity(chunks.len());
        let mut slots = BTreeMap::new();
        for &(pos, mesh, light) in chunks {
            let relative_origin = relative_chunk_origin(pos, camera_anchor)?;
            let slot = u16::try_from(metadata.len())
                .map_err(|_| RendererError::TooManyChunks(chunks.len()))?;
            let light_word_offset =
                u32::try_from(light_words.len()).map_err(|_| RendererError::LightDataTooLarge)?;
            pack_light_words(light, &mut light_words)?;
            metadata.push(ChunkMeta {
                relative_origin,
                light_info: [light_word_offset, 0, 0, 0],
            });
            let opaque_start =
                u32::try_from(quads.len()).map_err(|_| RendererError::MeshTooLarge)?;
            quads.extend(
                mesh.opaque
                    .iter()
                    .copied()
                    .map(|quad| GpuQuad::from(quad).with_chunk_slot(slot)),
            );
            let cutout_start =
                u32::try_from(quads.len()).map_err(|_| RendererError::MeshTooLarge)?;
            quads.extend(
                mesh.cutout
                    .iter()
                    .copied()
                    .map(|quad| GpuQuad::from(quad).with_chunk_slot(slot)),
            );
            let translucent_start =
                u32::try_from(quads.len()).map_err(|_| RendererError::MeshTooLarge)?;
            quads.extend(
                mesh.translucent
                    .iter()
                    .copied()
                    .map(|quad| GpuQuad::from(quad).with_chunk_slot(slot)),
            );
            let end = u32::try_from(quads.len()).map_err(|_| RendererError::MeshTooLarge)?;
            if padded {
                quads.resize(quads.len() + reserve(mesh, light), GpuQuad::zeroed());
            }
            let slot_end = u32::try_from(quads.len()).map_err(|_| RendererError::MeshTooLarge)?;
            slots.insert(
                pos,
                ResidentChunkSlot {
                    index: chunk_draws.len(),
                    start: opaque_start,
                    end: slot_end,
                    light_word_offset,
                },
            );
            chunk_draws.push(pipelines::ChunkLayerDraws {
                origin: relative_origin,
                opaque: opaque_start..cutout_start,
                cutout: cutout_start..translucent_start,
                translucent: translucent_start..end,
            });
        }
        if metadata.is_empty() {
            metadata.push(ChunkMeta::zeroed());
        }
        if light_words.is_empty() {
            light_words.push(u32::from_le_bytes([0xf0; 4]));
        }
        let mut camera_bindings_changed = pipelines::write_or_grow_buffer(
            &self.device,
            &self.queue,
            &mut self.chunk_meta_buffer,
            &mut self.chunk_meta_capacity,
            &metadata,
            wgpu::BufferUsages::STORAGE,
            "Voxy chunk metadata",
        );
        camera_bindings_changed |= pipelines::write_or_grow_buffer(
            &self.device,
            &self.queue,
            &mut self.light_buffer,
            &mut self.light_words_capacity,
            &light_words,
            wgpu::BufferUsages::STORAGE,
            "Voxy resident packed light",
        );
        pipelines::write_or_grow_buffer(
            &self.device,
            &self.queue,
            &mut self.quad_buffer,
            &mut self.quad_capacity,
            &quads,
            wgpu::BufferUsages::VERTEX,
            "Voxy resident chunk quads",
        );
        if camera_bindings_changed {
            self.camera_bind_group = create_camera_bind_group(
                &self.device,
                &self.camera_layout,
                &self.camera_buffer,
                &self.chunk_meta_buffer,
                &self.light_buffer,
            );
            self.photoreal_camera_bind_group = create_camera_bind_group(
                &self.device,
                &self.photoreal_camera_layout,
                &self.photoreal_camera_buffer,
                &self.chunk_meta_buffer,
                &self.light_buffer,
            );
            if let Some(shared) = &mut self.shared_camera {
                shared.refresh(
                    &self.device,
                    &self.camera_layout,
                    &self.chunk_meta_buffer,
                    &self.light_buffer,
                );
            }
            if let Some(shared) = &mut self.shared_photoreal_camera {
                shared.refresh(
                    &self.device,
                    &self.photoreal_camera_layout,
                    &self.chunk_meta_buffer,
                    &self.light_buffer,
                );
            }
        }
        self.chunk_upload_bytes = size_of_val(quads.as_slice())
            + size_of_val(light_words.as_slice())
            + size_of_val(metadata.as_slice());
        self.quad_count = quad_count;
        self.chunk_draws = chunk_draws;
        self.resident_chunk_slots = slots;
        if self.resident_camera_anchor != camera_anchor {
            self.reset_skinned_motion();
            self.resident_camera_anchor = camera_anchor;
        }
        Ok(())
    }

    pub fn set_material_pack(&mut self, pack: MaterialPack) {
        if self.device_failure().is_some() {
            return;
        }
        let set = MaterialSet::upload(&self.device, &self.queue, &pack);
        self.material_bind_group =
            create_material_bind_group(&self.device, &self.material_layout, &set);
        self.material_pack = pack;
        self.material_set = set;
    }

    /// Updates the view-projection uniform without reallocating GPU resources.
    ///
    /// # Errors
    ///
    /// Rejects non-finite or degenerate camera parameters.
    pub fn update_camera(&mut self, camera: CameraView) -> Result<(), RendererError> {
        self.check_device()?;
        if !camera.eye.is_finite()
            || !camera.target.is_finite()
            || !camera.up.is_finite()
            || camera.eye.distance_squared(camera.target) <= f32::EPSILON
            || camera.up.length_squared() <= f32::EPSILON
            || !camera.vertical_fov_radians.is_finite()
            || !(0.01..3.13).contains(&camera.vertical_fov_radians)
            || !camera.near_plane.is_finite()
            || camera.near_plane <= 0.0
        {
            return Err(RendererError::InvalidCamera);
        }
        self.camera = camera;
        if self.shared_camera.is_none() {
            self.queue.write_buffer(
                &self.camera_buffer,
                0,
                bytemuck::bytes_of(&camera_uniform(
                    self.config.width,
                    self.config.height,
                    camera,
                )),
            );
        }
        if self.shared_photoreal_camera.is_none() {
            self.queue.write_buffer(
                &self.photoreal_camera_buffer,
                0,
                bytemuck::bytes_of(&photoreal_camera_uniform(
                    self.config.width,
                    self.config.height,
                    camera,
                    &self.bodycam_rig,
                    self.bodycam_flashlight_pos,
                    self.bodycam_flashlight_dir,
                    self.bodycam_wetness,
                )),
            );
        }
        Ok(())
    }

    /// Replaces the single currently resident animated mesh and joint palette.
    ///
    /// # Errors
    ///
    /// Rejects invalid/singular world deformation, palette mismatch or excessive indices.
    pub fn upload_skinned_mesh(
        &mut self,
        mesh: &SkinnedMesh,
        joints: &[Mat4],
        model: Mat4,
        material_layer: u32,
    ) -> Result<(), RendererError> {
        self.check_device()?;
        self.skinned = Some(
            upload_skinned(
                &self.device,
                &self.skin_layout,
                mesh,
                joints,
                model,
                material_layer,
            )
            .map_err(RendererError::Skinned)?,
        );
        self.skinned_lod = None;
        self.skinned_camera_history.reset();
        self.skinned_motion_output = None;
        self.scene_depth_motion = None;
        self.skinned_motion_metadata = None;
        self.skinned_motion = Some(crate::skinned_motion::ResidentSkinnedMotion {
            history: crate::SkinnedMotionHistory::new(mesh.clone()),
            joints: joints.to_vec(),
            model,
        });
        Ok(())
    }

    /// Prepare deformation correspondence against the last presented resident pose.
    /// First/reset frames report invalid history; propagate that to temporal consumers.
    /// # Errors
    /// Reports missing geometry or unsupported/nonfinite pose transforms.
    pub fn prepare_skinned_motion(&self) -> Result<crate::SkinnedMotionFrame, RendererError> {
        let motion = self
            .skinned_motion
            .as_ref()
            .ok_or(RendererError::NoSkinnedMesh)?;
        motion
            .history
            .prepare(&motion.joints, motion.model)
            .map_err(RendererError::Skinned)
    }
    /// Current/last-presented unjittered camera for resident skeletal motion.
    /// Uses the same reverse-Z projection and view as the primary renderer.
    /// # Errors
    /// Rejects nonfinite camera matrices without advancing history.
    pub fn prepare_skinned_motion_camera(
        &self,
    ) -> Result<crate::MotionMatrices, crate::InvalidMotionMatrix> {
        self.skinned_camera_history.prepare(camera_view_projection(
            self.config.width,
            self.config.height,
            self.camera,
        ))
    }
    /// Enable automatic resident skeletal RG16 motion output on presented frames.
    /// Currently recreates paired geometry each frame; defaults to disabled.
    pub fn set_skinned_motion_enabled(&mut self, enabled: bool) {
        if self.skinned_motion_enabled != enabled {
            self.skinned_motion_enabled = enabled;
            self.reset_skinned_motion();
        }
    }
    /// Last presented motion texture: skeletal motion plus static-world camera motion.
    /// Resize/reset/replacement clears access; moving non-skeletal geometry still requires its own correspondence.
    #[must_use]
    pub fn skinned_motion_texture(&self) -> Option<&wgpu::Texture> {
        self.scene_depth_motion
            .as_ref()
            .map(crate::DepthMotionPass::output)
    }
    /// Motion and reset metadata share the same successfully presented frame ID.
    #[must_use]
    pub fn skinned_motion_output(&self) -> Option<SkinnedMotionOutput<'_>> {
        let (presentation_id, reset_history) = self.skinned_motion_metadata?;
        Some(SkinnedMotionOutput {
            presentation_id,
            reset_history,
            motion: self.skinned_motion_texture()?,
        })
    }
    fn create_skinned_motion_pass(
        &self,
    ) -> Result<
        (
            Option<crate::RasterMotionPass>,
            Option<crate::DepthMotionPass>,
            bool,
        ),
        RendererError,
    > {
        if !self.skinned_motion_enabled || self.skinned_motion.is_none() {
            return Ok((None, None, true));
        }
        let frame = self.prepare_skinned_motion()?;
        let camera = self
            .prepare_skinned_motion_camera()
            .map_err(|_| RendererError::InvalidCamera)?;
        let reset = !frame.history_valid || !camera.history_valid;
        let cameras = [camera.current, camera.previous];
        let candidate = if let Some(previous) = &self.skinned_motion_output {
            previous.next_frame(
                &self.device,
                &self.depth_texture,
                cameras,
                &frame.vertices,
                reset,
            )
        } else {
            crate::RasterMotionPass::new(
                &self.device,
                &self.depth_texture,
                cameras,
                &frame.vertices,
                reset,
            )
        };
        let pass = candidate.map_err(RendererError::Ray)?;
        let base =
            crate::DepthMotionPass::new(&self.device, &self.depth_texture, cameras, 0.0, reset)
                .map_err(RendererError::Ray)?;
        Ok((Some(pass), Some(base), reset))
    }
    /// Invalidate animated temporal history after a camera cut or teleport.
    pub fn reset_skinned_motion(&mut self) {
        self.skinned_motion_output = None;
        self.scene_depth_motion = None;
        self.skinned_motion_metadata = None;
        self.skinned_camera_history.reset();
        if let Some(motion) = &mut self.skinned_motion {
            motion.history.reset();
        }
    }

    /// Updates only the animated joint palette without reallocating geometry.
    ///
    /// # Errors
    ///
    /// Rejects missing geometry or invalid/singular deformation before GPU writes.
    pub fn update_skin_matrices(&mut self, joints: &[Mat4]) -> Result<(), RendererError> {
        self.check_device()?;
        let lod_pose = self
            .skinned_lod
            .as_ref()
            .map(|lod| {
                lod.pose
                    .source()
                    .prepare(joints, lod.pose.model())
                    .map_err(|error| {
                        RendererError::SkinnedLod(crate::SkinnedLodGpuError::Pose(error))
                    })
            })
            .transpose()?;
        let skinned = self.skinned.as_ref().ok_or(RendererError::NoSkinnedMesh)?;
        if joints.len() != skinned.joint_count {
            return Err(RendererError::Skinned(
                SkinnedUploadError::JointCountMismatch,
            ));
        }
        if joints.iter().any(|matrix| !matrix.is_finite()) {
            return Err(RendererError::Skinned(SkinnedUploadError::NonFiniteMatrix));
        }
        let motion = self
            .skinned_motion
            .as_ref()
            .ok_or(RendererError::NoSkinnedMesh)?;
        skinned
            .write_pose(&self.queue, motion.history.mesh(), joints, motion.model)
            .map_err(RendererError::Skinned)?;
        if let Some(motion) = &mut self.skinned_motion {
            motion.joints.clone_from_slice(joints);
        }
        if let Some(pose) = lod_pose {
            self.skinned_lod
                .as_mut()
                .ok_or(RendererError::NoSkinnedLod)?
                .pose = pose;
            self.bind_skinned_lod_level(0)?;
        }
        Ok(())
    }

    /// Updates joints and world transform together: one pose validation and one
    /// pair of GPU writes, where calling [`Self::update_skin_matrices`] followed by
    /// [`Self::update_skinned_model`] validates and writes twice per tick.
    ///
    /// # Errors
    ///
    /// Rejects missing geometry or invalid/singular deformation before GPU writes.
    pub fn update_skinned_pose(
        &mut self,
        joints: &[Mat4],
        model: Mat4,
    ) -> Result<(), RendererError> {
        self.check_device()?;
        if !model.is_finite() {
            return Err(RendererError::Skinned(SkinnedUploadError::NonFiniteMatrix));
        }
        let lod_pose = self
            .skinned_lod
            .as_ref()
            .map(|lod| {
                lod.pose.source().prepare(joints, model).map_err(|error| {
                    RendererError::SkinnedLod(crate::SkinnedLodGpuError::Pose(error))
                })
            })
            .transpose()?;
        let skinned = self.skinned.as_ref().ok_or(RendererError::NoSkinnedMesh)?;
        if joints.len() != skinned.joint_count {
            return Err(RendererError::Skinned(
                SkinnedUploadError::JointCountMismatch,
            ));
        }
        if joints.iter().any(|matrix| !matrix.is_finite()) {
            return Err(RendererError::Skinned(SkinnedUploadError::NonFiniteMatrix));
        }
        let motion = self
            .skinned_motion
            .as_ref()
            .ok_or(RendererError::NoSkinnedMesh)?;
        self.pose_timing = skinned
            .write_pose_timed(
                &self.queue,
                motion.history.mesh(),
                joints,
                model,
                self.profile_pose,
            )
            .map_err(RendererError::Skinned)?;
        if let Some(motion) = &mut self.skinned_motion {
            motion.joints.clone_from_slice(joints);
            motion.model = model;
        }
        if let Some(pose) = lod_pose {
            self.skinned_lod
                .as_mut()
                .ok_or(RendererError::NoSkinnedLod)?
                .pose = pose;
            self.bind_skinned_lod_level(0)?;
        }
        Ok(())
    }

    /// Take the latest measured pose admission and queue write durations.
    pub fn take_profile_pose_timing(
        &mut self,
    ) -> Option<(crate::RenderPoseProfile, std::time::Duration)> {
        self.pose_timing.take()
    }

    /// Updates the animated object's world transform without rebuilding its mesh.
    ///
    /// # Errors
    ///
    /// Rejects missing geometry or invalid/singular deformation before GPU writes.
    pub fn update_skinned_model(&mut self, model: Mat4) -> Result<(), RendererError> {
        self.check_device()?;
        if !model.is_finite() {
            return Err(RendererError::Skinned(SkinnedUploadError::NonFiniteMatrix));
        }
        let lod_pose = self
            .skinned_lod
            .as_ref()
            .map(|lod| {
                lod.pose
                    .source()
                    .prepare(lod.pose.joints(), model)
                    .map_err(|error| {
                        RendererError::SkinnedLod(crate::SkinnedLodGpuError::Pose(error))
                    })
            })
            .transpose()?;
        let skinned = self.skinned.as_ref().ok_or(RendererError::NoSkinnedMesh)?;
        let motion = self
            .skinned_motion
            .as_ref()
            .ok_or(RendererError::NoSkinnedMesh)?;
        skinned
            .write_pose(&self.queue, motion.history.mesh(), &motion.joints, model)
            .map_err(RendererError::Skinned)?;
        if let Some(motion) = &mut self.skinned_motion {
            motion.model = model;
        }
        if let Some(pose) = lod_pose {
            self.skinned_lod
                .as_mut()
                .ok_or(RendererError::NoSkinnedLod)?
                .pose = pose;
            self.bind_skinned_lod_level(0)?;
        }
        Ok(())
    }

    /// Acquires, clears, submits, and presents one surface image.
    ///
    /// # Errors
    ///
    /// Returns surface/device-loss errors when native graphics must be recreated.
    pub fn render(&mut self) -> Result<RenderOutcome, RendererError> {
        self.check_device()?;
        self.profile_acquire = std::time::Duration::ZERO;
        self.profile_gpu = [None; 4];
        if let Some(timing) = &mut self.frame_timing {
            let _ = self.device.poll(wgpu::PollType::Poll);
            self.profile_gpu = timing.collect(self.queue.get_timestamp_period());
        }
        if self.state == SurfaceState::Suspended {
            return Ok(RenderOutcome::Suspended);
        }
        let frame_id = self
            .resident_presented_frames
            .checked_add(1)
            .ok_or(RendererError::FrameIdExhausted)?;
        let acquire_start = std::time::Instant::now();
        let acquired = self.surface.get_current_texture();
        self.profile_acquire = acquire_start.elapsed();
        let (frame, suboptimal) = match acquired {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.reset_skinned_motion();
                self.surface.configure(&self.device, &self.config);
                return Ok(RenderOutcome::Reconfigured);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.reset_skinned_motion();
                return Err(RendererError::SurfaceLost);
            }
            wgpu::CurrentSurfaceTexture::Timeout => return Ok(RenderOutcome::SkippedTimeout),
            wgpu::CurrentSurfaceTexture::Occluded => return Ok(RenderOutcome::SkippedOccluded),
            wgpu::CurrentSurfaceTexture::Validation => {
                self.reset_skinned_motion();
                return Ok(RenderOutcome::SkippedValidation);
            }
        };
        self.refresh_skinned_lod()?;
        let (motion_pass, base_motion, reset_motion) = self.create_skinned_motion_pass()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Voxy frame encoder"),
            });
        if self.bodycam_enabled {
            self.encode_bodycam_passes(&mut encoder, &view, frame_id);
        } else {
            self.encode_resident_voxels(&mut encoder, &view);
        }
        encode_motion_layers(&mut encoder, motion_pass.as_ref(), base_motion.as_ref())?;
        let timed_passes = if self.native_vegetation.is_some() {
            4
        } else {
            1
        };
        if let Some(timing) = &self.frame_timing {
            timing.resolve(&mut encoder, timed_passes);
        }
        self.queue.submit([encoder.finish()]);
        if let Some(timing) = &mut self.frame_timing {
            timing.map(timed_passes);
        }
        self.queue.present(frame);
        self.resident_presented_frames = frame_id;
        self.skinned_motion_metadata = motion_pass.as_ref().map(|_| (frame_id, reset_motion));
        self.skinned_motion_output = motion_pass;
        self.scene_depth_motion = base_motion;
        self.commit_skinned_temporal_history();
        if suboptimal {
            self.surface.configure(&self.device, &self.config);
            Ok(RenderOutcome::Reconfigured)
        } else {
            Ok(RenderOutcome::Presented)
        }
    }

    pub(crate) fn encode_resident_voxels(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        self.encode_resident_voxels_to_targets(encoder, view, &self.depth_view, true);
    }

    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub(crate) fn encode_resident_voxels_to_targets(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        clear: bool,
    ) {
        let camera_group = self.shared_camera.as_ref().map_or_else(
            || self.camera_bind_group.clone(),
            |shared| {
                shared.select(
                    encoder,
                    &self.camera_buffer,
                    &self.camera_bind_group,
                    bytemuck::bytes_of(&camera_uniform(
                        self.config.width,
                        self.config.height,
                        self.camera,
                    )),
                )
            },
        );
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Voxy voxel pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: if clear {
                            wgpu::LoadOp::Clear(self.clear_color)
                        } else {
                            wgpu::LoadOp::Load
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: if clear {
                            wgpu::LoadOp::Clear(0.0)
                        } else {
                            wgpu::LoadOp::Load
                        },
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: self
                    .frame_timing
                    .as_ref()
                    .and_then(|timing| timing.writes(0)),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let view_projection =
                camera_view_projection(self.config.width, self.config.height, self.camera);
            if self.quad_count != 0 {
                pipelines::encode_voxel_layers(
                    &mut pass,
                    &self.pipeline,
                    &camera_group,
                    &self.material_bind_group,
                    &self.quad_buffer,
                    &self.chunk_draws,
                    view_projection,
                    self.camera.eye,
                    false,
                );
            }
            if let Some(skinned) = &self.skinned {
                pass.set_pipeline(&self.skinned_pipeline);
                pass.set_bind_group(0, &camera_group, &[]);
                pass.set_bind_group(1, &self.material_bind_group, &[]);
                pass.set_bind_group(2, &skinned.bind_group, &[]);
                pass.set_vertex_buffer(0, skinned.vertex.slice(..));
                pass.set_index_buffer(skinned.index.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..skinned.index_count, 0, 0..1);
            }
            if let Some(indirect) = &self.indirect_mesh {
                pass.set_pipeline(&self.indirect_pipeline);
                pass.set_bind_group(0, &camera_group, &[]);
                pass.set_bind_group(1, &self.material_bind_group, &[]);
                pass.set_vertex_buffer(0, indirect.vertex_buffer.slice(..));
                pass.draw_indirect(&indirect.indirect_buffer, 0);
            }
            if self.quad_count != 0 {
                pipelines::encode_voxel_layers(
                    &mut pass,
                    &self.pipeline,
                    &camera_group,
                    &self.material_bind_group,
                    &self.quad_buffer,
                    &self.chunk_draws,
                    view_projection,
                    self.camera.eye,
                    true,
                );
            }
        }
        if let Some(plants) = &self.native_vegetation {
            plants.encode(
                &self.device,
                &self.queue,
                encoder,
                view,
                depth_view,
                camera_view_projection(self.config.width, self.config.height, self.camera),
                self.camera.eye,
                false,
                self.vegetation_timestamps(),
            );
        }
    }

    pub(super) fn vegetation_timestamps(&self) -> [Option<wgpu::RenderPassTimestampWrites<'_>>; 3] {
        [1, 2, 3].map(|pass| {
            self.frame_timing
                .as_ref()
                .and_then(|timing| timing.writes(pass))
        })
    }

    fn commit_skinned_temporal_history(&mut self) {
        let pose_valid = self
            .skinned_motion
            .as_mut()
            .is_none_or(crate::skinned_motion::ResidentSkinnedMotion::commit_presented);
        if !pose_valid
            || self
                .skinned_camera_history
                .presented(camera_view_projection(
                    self.config.width,
                    self.config.height,
                    self.camera,
                ))
                .is_err()
        {
            self.reset_skinned_motion();
        }
    }
}

#[path = "renderer/pipelines.rs"]
mod pipelines;
pub use pipelines::RendererError;
use pipelines::{
    create_camera_bind_group, create_camera_layout, create_depth, create_indirect_pipeline,
    create_material_bind_group, create_material_layout, create_photoreal_camera_layout,
    create_photoreal_pipelines, create_pipeline, default_material_pack, encode_motion_layers,
};

#[cfg(test)]
mod tests {
    use std::num::NonZeroU8;

    use super::*;
    use voxy_mesher::MaterialId;

    #[test]
    fn gpu_quad_has_stable_pod_layout() {
        assert_eq!(size_of::<GpuQuad>(), 20);
        assert_eq!(align_of::<GpuQuad>(), 2);
        let quad = Quad {
            origin: [1, 2, 3],
            extent_u: NonZeroU8::new(4).unwrap(),
            extent_v: NonZeroU8::new(5).unwrap(),
            face: FaceDir::PosZ,
            material: MaterialId(9),
            layer: RenderLayer::Cutout,
            ao: [0, 1, 2, 3],
            diagonal: QuadDiagonal::Vu,
        };
        let gpu = GpuQuad::from(quad);
        assert_eq!(gpu.origin, [1, 2, 3, 0]);
        assert_eq!(gpu.extent_face, [4, 5, 5, 0]);
        assert_eq!(gpu.material_layer, [9, 1]);
        assert_eq!(gpu.ao_diagonal, [0, 1, 2, 131]);
        assert_eq!(gpu.chunk_slot, [0, 0]);
        assert_eq!(bytemuck::bytes_of(&gpu).len(), 20);
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "requires a physical Metal GPU"]
    fn resident_slot_metal_readback_preserves_neighbors_and_unused_tail() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let info = adapter.get_info();
        eprintln!("Resident chunk readback on {info:?}");
        assert_eq!(info.backend, wgpu::Backend::Metal);
        assert_ne!(info.device_type, wgpu::DeviceType::Cpu);
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let slot = ResidentChunkSlot {
            index: 1,
            start: 2,
            end: 4,
            light_word_offset: 4,
        };
        let replacement = GpuQuad::from(Quad {
            origin: [7, 8, 9],
            extent_u: NonZeroU8::MIN,
            extent_v: NonZeroU8::MIN,
            face: FaceDir::PosZ,
            material: MaterialId(9),
            layer: RenderLayer::Opaque,
            ao: [0, 1, 2, 3],
            diagonal: QuadDiagonal::Uv,
        })
        .with_chunk_slot(1);
        let light_words = [0x1234_abcd_u32; 4];
        let mut expected_quads = [0x5a; 6 * size_of::<GpuQuad>()];
        let mut expected_light = [0xa5; 12 * size_of::<u32>()];
        let create = |size: usize, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: size as u64,
                usage,
                mapped_at_creation: false,
            })
        };
        let copy = wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST;
        let quads = create(expected_quads.len(), copy | wgpu::BufferUsages::VERTEX);
        let light = create(expected_light.len(), copy | wgpu::BufferUsages::STORAGE);
        queue.write_buffer(&quads, 0, &expected_quads);
        queue.write_buffer(&light, 0, &expected_light);
        slot.write_update(&queue, &quads, &light, &[replacement], &light_words);

        expected_quads[40..60].copy_from_slice(bytemuck::bytes_of(&replacement));
        expected_light[16..32].copy_from_slice(bytemuck::cast_slice(&light_words));
        let readback = create(
            expected_quads.len() + expected_light.len(),
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        );
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&quads, 0, &readback, 0, quads.size());
        encoder.copy_buffer_to_buffer(&light, 0, &readback, quads.size(), light.size());
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        rx.recv().unwrap().unwrap();
        let bytes = slice.get_mapped_range().unwrap();
        let (actual_quads, actual_light) = bytes.split_at(expected_quads.len());
        assert_eq!(actual_quads, expected_quads);
        assert_eq!(actual_light, expected_light);
    }

    #[test]
    fn resident_slot_accepts_shrink_and_layer_changes_within_its_range() {
        let slot = ResidentChunkSlot {
            index: 3,
            start: 10,
            end: 15,
            light_word_offset: 48,
        };
        let quad = Quad {
            origin: [0; 3],
            extent_u: NonZeroU8::MIN,
            extent_v: NonZeroU8::MIN,
            face: FaceDir::PosZ,
            material: MaterialId(1),
            layer: RenderLayer::Opaque,
            ao: [0; 4],
            diagonal: QuadDiagonal::Uv,
        };
        let mut mesh = ChunkMesh {
            opaque: vec![quad; 2].into_boxed_slice(),
            cutout: Box::new([]),
            translucent: vec![quad].into_boxed_slice(),
            bounds: voxy_mesher::LocalAabb::EMPTY,
        };
        let draw = slot.draw_ranges(&mesh, [-32, 0, 0, 0]).unwrap();
        assert_eq!(draw.origin, [-32, 0, 0, 0]);
        assert_eq!(draw.opaque, 10..12);
        assert_eq!(draw.cutout, 12..12);
        assert_eq!(draw.translucent, 12..13);
        mesh.cutout = vec![quad; 2].into_boxed_slice();
        assert_eq!(slot.draw_ranges(&mesh, [0; 4]).unwrap().translucent, 14..15);
        mesh.cutout = vec![quad; 3].into_boxed_slice();
        assert!(slot.draw_ranges(&mesh, [0; 4]).is_none());
    }

    #[test]
    fn clip_space_aabb_culling_rejects_fully_outside_chunks() {
        assert!(chunk_intersects_frustum(Mat4::IDENTITY, [0, 0, 0, 0]));
        assert!(!chunk_intersects_frustum(Mat4::IDENTITY, [2, 0, 0, 0]));
        assert!(!chunk_intersects_frustum(Mat4::IDENTITY, [0, 0, -33, 0]));
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    fn noop_backend_validates_voxel_pipeline_and_draw() {
        let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor {
            label: Some("Voxy validation device"),
            ..Default::default()
        });
        let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let camera_layout = create_camera_layout(&device);
        let material_layout = create_material_layout(&device);
        let pipeline = create_pipeline(
            &device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            &camera_layout,
            &material_layout,
        );
        let skin_layout = create_skin_layout(&device);
        let skinned_pipeline = create_skinned_pipeline(
            &device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            &camera_layout,
            &material_layout,
            &skin_layout,
        );
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("test camera"),
            contents: bytemuck::bytes_of(&camera_uniform(640, 480, CameraView::default())),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let chunk_meta = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("test chunk metadata"),
            contents: bytemuck::bytes_of(&ChunkMeta::zeroed()),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let light_data = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("test light data"),
            contents: bytemuck::bytes_of(&u32::from_le_bytes([0xf0; 4])),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let camera_bind = create_camera_bind_group(
            &device,
            &camera_layout,
            &camera_buffer,
            &chunk_meta,
            &light_data,
        );
        let pack = default_material_pack();
        let materials = MaterialSet::upload(&device, &queue, &pack);
        let material_bind = create_material_bind_group(&device, &material_layout, &materials);
        let skinned_mesh = SkinnedMesh::new(
            vec![
                crate::SkinnedVertex {
                    position: [0.0, 0.0, 0.0],
                    normal: [0.0, 1.0, 0.0],
                    uv: [0.0, 0.0],
                    joints: [0; 4],
                    weights: [u16::MAX, 0, 0, 0],
                },
                crate::SkinnedVertex {
                    position: [1.0, 0.0, 0.0],
                    normal: [0.0, 1.0, 0.0],
                    uv: [1.0, 0.0],
                    joints: [0; 4],
                    weights: [u16::MAX, 0, 0, 0],
                },
                crate::SkinnedVertex {
                    position: [0.0, 1.0, 0.0],
                    normal: [0.0, 1.0, 0.0],
                    uv: [0.0, 1.0],
                    joints: [0; 4],
                    weights: [u16::MAX, 0, 0, 0],
                },
            ],
            vec![0, 1, 2],
            1,
        )
        .unwrap();
        let skinned = upload_skinned(
            &device,
            &skin_layout,
            &skinned_mesh,
            &[Mat4::IDENTITY],
            Mat4::IDENTITY,
            1,
        )
        .unwrap();
        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test color"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());
        let (_depth, depth_view) = create_depth(&device, 64, 64);
        let quad = GpuQuad {
            origin: [0, 0, 0, 0],
            extent_face: [1, 1, 5, 0],
            material_layer: [1, 0],
            ao_diagonal: [3; 4],
            chunk_slot: [0, 0],
        };
        let quads = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("test quad"),
            contents: bytemuck::bytes_of(&quad),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("test voxel encoder"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("test voxel pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&pipeline.opaque);
            pass.set_bind_group(0, &camera_bind, &[]);
            pass.set_bind_group(1, &material_bind, &[]);
            pass.set_vertex_buffer(0, quads.slice(..));
            pass.draw(0..6, 0..1);
            pass.set_pipeline(&pipeline.cutout);
            pass.draw(0..6, 0..1);
            pass.set_pipeline(&pipeline.translucent);
            pass.draw(0..6, 0..1);
            pass.set_pipeline(&skinned_pipeline);
            pass.set_bind_group(0, &camera_bind, &[]);
            pass.set_bind_group(1, &material_bind, &[]);
            pass.set_bind_group(2, &skinned.bind_group, &[]);
            pass.set_vertex_buffer(0, skinned.vertex.slice(..));
            pass.set_index_buffer(skinned.index.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..skinned.index_count, 0, 0..1);
        }
        queue.submit([encoder.finish()]);
        let validation_error = pollster::block_on(error_scope.pop());
        assert!(validation_error.is_none(), "{validation_error:?}");
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    fn noop_backend_validates_indirect_voxel_pipeline_and_draw() {
        let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor {
            label: Some("Voxy validation device indirect"),
            required_limits: wgpu::Limits {
                min_storage_buffer_offset_alignment: 256,
                ..Default::default()
            },
            ..Default::default()
        });
        let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let camera_layout = create_camera_layout(&device);
        let material_layout = create_material_layout(&device);
        let indirect_pipeline = create_indirect_pipeline(
            &device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            &camera_layout,
            &material_layout,
        );
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("test camera"),
            contents: bytemuck::bytes_of(&camera_uniform(640, 480, CameraView::default())),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let chunk_meta = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("test chunk metadata"),
            contents: bytemuck::bytes_of(&ChunkMeta::zeroed()),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let light_data = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("test light data"),
            contents: bytemuck::bytes_of(&u32::from_le_bytes([0xf0; 4])),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let camera_bind = create_camera_bind_group(
            &device,
            &camera_layout,
            &camera_buffer,
            &chunk_meta,
            &light_data,
        );
        let pack = default_material_pack();
        let materials = MaterialSet::upload(&device, &queue, &pack);
        let material_bind = create_material_bind_group(&device, &material_layout, &materials);

        let vertex_data = [0u8; 32 * 3];
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("test indirect vertex"),
            contents: &vertex_data,
            usage: wgpu::BufferUsages::VERTEX,
        });

        let indirect_args = [3u32, 1, 0, 0]; // 3 vertices, 1 instance
        let _indirect_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("test indirect args"),
            contents: bytemuck::cast_slice(&indirect_args),
            usage: wgpu::BufferUsages::INDIRECT,
        });

        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test color"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());
        let (_depth, depth_view) = create_depth(&device, 64, 64);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("test indirect encoder"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("test indirect pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&indirect_pipeline);
            pass.set_bind_group(0, &camera_bind, &[]);
            pass.set_bind_group(1, &material_bind, &[]);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.draw(0..3, 0..1);
        }
        queue.submit([encoder.finish()]);
        let validation_error = pollster::block_on(error_scope.pop());
        assert!(validation_error.is_none(), "{validation_error:?}");
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    fn noop_backend_validates_photoreal_bodycam_pipelines_and_draw() {
        let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor {
            label: Some("Voxy validation device photoreal"),
            ..Default::default()
        });
        let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let photoreal_camera_layout = create_photoreal_camera_layout(&device);
        let material_layout = create_material_layout(&device);
        let skin_layout = create_skin_layout(&device);
        let (photoreal_pipeline, photoreal_indirect, photoreal_skinned, photoreal_fibres) =
            create_photoreal_pipelines(
                &device,
                wgpu::TextureFormat::Rgba16Float,
                &photoreal_camera_layout,
                &material_layout,
                &skin_layout,
            );
        let rig = BodycamRig::default();
        let cam_uniform = photoreal_camera_uniform(
            320,
            180,
            CameraView::default(),
            &rig,
            Vec3::new(10.0, 4.0, 10.0),
            Vec3::new(0.0, 0.0, -1.0),
            0.7,
        );
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("test photoreal camera"),
            contents: bytemuck::bytes_of(&cam_uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let chunk_meta = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("test chunk metadata"),
            contents: bytemuck::bytes_of(&ChunkMeta::zeroed()),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let light_data = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("test light data"),
            contents: bytemuck::bytes_of(&u32::from_le_bytes([0xf0; 4])),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let camera_bind = create_camera_bind_group(
            &device,
            &photoreal_camera_layout,
            &camera_buffer,
            &chunk_meta,
            &light_data,
        );
        let pack = default_material_pack();
        let materials = MaterialSet::upload(&device, &queue, &pack);
        let material_bind = create_material_bind_group(&device, &material_layout, &materials);
        let (_depth, depth_view) = create_depth(&device, 320, 180);
        let mut post =
            BodycamPostProcess::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb, 320, 180);
        post.update_from_rig(&queue, &rig, 1);

        let quad = GpuQuad {
            origin: [0, 0, 0, 0],
            extent_face: [1, 1, 3, 0],
            material_layer: [1, 0],
            ao_diagonal: [3; 4],
            chunk_slot: [0, 0],
        };
        let quads = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("test quad"),
            contents: bytemuck::bytes_of(&quad),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indirect_verts = [0u8; 32 * 3];
        let indirect_vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("test indirect vb"),
            contents: &indirect_verts,
            usage: wgpu::BufferUsages::VERTEX,
        });
        let skinned_mesh = SkinnedMesh::new(
            vec![
                crate::SkinnedVertex {
                    position: [0.0, 0.0, 0.0],
                    normal: [0.0, 1.0, 0.0],
                    uv: [0.0, 0.0],
                    joints: [0; 4],
                    weights: [u16::MAX, 0, 0, 0],
                },
                crate::SkinnedVertex {
                    position: [1.0, 0.0, 0.0],
                    normal: [0.0, 1.0, 0.0],
                    uv: [1.0, 0.0],
                    joints: [0; 4],
                    weights: [u16::MAX, 0, 0, 0],
                },
                crate::SkinnedVertex {
                    position: [0.0, 1.0, 0.0],
                    normal: [0.0, 1.0, 0.0],
                    uv: [0.0, 1.0],
                    joints: [0; 4],
                    weights: [u16::MAX, 0, 0, 0],
                },
            ],
            vec![0, 1, 2],
            1,
        )
        .unwrap();
        let skinned = upload_skinned(
            &device,
            &skin_layout,
            &skinned_mesh,
            &[Mat4::IDENTITY],
            Mat4::IDENTITY,
            1,
        )
        .unwrap();

        let out_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test srgb out"),
            size: wgpu::Extent3d {
                width: 320,
                height: 180,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let out_view = out_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("test photoreal hdr pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: post.hdr_view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&photoreal_pipeline.opaque);
            pass.set_bind_group(0, &camera_bind, &[]);
            pass.set_bind_group(1, &material_bind, &[]);
            pass.set_vertex_buffer(0, quads.slice(..));
            pass.draw(0..6, 0..1);
            pass.set_pipeline(&photoreal_pipeline.cutout);
            pass.draw(0..6, 0..1);
            pass.set_pipeline(&photoreal_pipeline.translucent);
            pass.draw(0..6, 0..1);

            pass.set_pipeline(&photoreal_indirect);
            pass.set_bind_group(0, &camera_bind, &[]);
            pass.set_bind_group(1, &material_bind, &[]);
            pass.set_vertex_buffer(0, indirect_vb.slice(..));
            pass.draw(0..3, 0..1);

            pass.set_pipeline(&photoreal_skinned);
            pass.set_bind_group(0, &camera_bind, &[]);
            pass.set_bind_group(1, &material_bind, &[]);
            pass.set_bind_group(2, &skinned.bind_group, &[]);
            pass.set_vertex_buffer(0, skinned.vertex.slice(..));
            pass.set_index_buffer(skinned.index.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..skinned.index_count, 0, 0..1);
            pass.set_pipeline(&photoreal_fibres);
            pass.draw_indexed(0..skinned.index_count, 0, 0..1);
        }
        post.encode(&mut encoder, &out_view);
        queue.submit([encoder.finish()]);
        let validation_error = pollster::block_on(error_scope.pop());
        assert!(validation_error.is_none(), "{validation_error:?}");
    }
}

#[cfg(test)]
#[path = "renderer/legacy_skin_tests.rs"]
mod legacy_skin_tests;

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "renderer_shared_camera_tests.rs"]
mod shared_camera_tests;
