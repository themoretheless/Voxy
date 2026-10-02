use std::fmt;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::camera::rh::{proj::directx::orthographic, view::look_at_mat4};
use glam::{Mat4, Vec3, Vec4};
use voxy_core::{CHUNK_EDGE, ChunkPos};
use voxy_lighting::LightVolume;
use voxy_mesher::{ChunkMesh, FaceDir, Quad, QuadDiagonal, RenderLayer};
use wgpu::util::DeviceExt;

use crate::skinned::{GpuSkinnedMesh, create_skin_layout, create_skinned_pipeline, upload_skinned};
use crate::{MaterialLayer, MaterialPack, MaterialSet};
use crate::{SkinnedMesh, SkinnedUploadError};
mod skinned_lod;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Pod, Zeroable)]
pub struct GpuQuad {
    pub origin: [u8; 4],
    pub extent_face: [u8; 4],
    pub material_layer: [u16; 2],
    pub ao_diagonal: [u8; 4],
    pub chunk_slot: [u16; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct ChunkMeta {
    relative_origin: [i32; 4],
    light_info: [u32; 4],
}

impl From<Quad> for GpuQuad {
    fn from(quad: Quad) -> Self {
        Self {
            origin: [quad.origin[0], quad.origin[1], quad.origin[2], 0],
            extent_face: [
                quad.extent_u.get(),
                quad.extent_v.get(),
                face_code(quad.face),
                0,
            ],
            material_layer: [quad.material.0, layer_code(quad.layer)],
            ao_diagonal: [
                quad.ao[0],
                quad.ao[1],
                quad.ao[2],
                (quad.ao[3] & 0x7f) | (u8::from(quad.diagonal == QuadDiagonal::Vu) << 7),
            ],
            chunk_slot: [0, 0],
        }
    }
}

impl GpuQuad {
    fn with_chunk_slot(mut self, chunk_slot: u16) -> Self {
        self.chunk_slot[0] = chunk_slot;
        self
    }
}

const fn face_code(face: FaceDir) -> u8 {
    match face {
        FaceDir::NegX => 0,
        FaceDir::PosX => 1,
        FaceDir::NegY => 2,
        FaceDir::PosY => 3,
        FaceDir::NegZ => 4,
        FaceDir::PosZ => 5,
    }
}

const fn layer_code(layer: RenderLayer) -> u16 {
    match layer {
        RenderLayer::Opaque => 0,
        RenderLayer::Cutout => 1,
        RenderLayer::Translucent => 2,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceState {
    Active,
    Suspended,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderOutcome {
    Presented,
    Reconfigured,
    SkippedTimeout,
    SkippedOccluded,
    SkippedValidation,
    Suspended,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraView {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub vertical_fov_radians: f32,
    pub near_plane: f32,
}

impl Default for CameraView {
    fn default() -> Self {
        Self {
            eye: Vec3::new(52.0, 38.0, 52.0),
            target: Vec3::new(16.0, 5.0, 16.0),
            up: Vec3::Y,
            vertical_fov_radians: 55_f32.to_radians(),
            near_plane: 0.1,
        }
    }
}

/// Partial-scene skeletal motion from one successfully presented resident frame.
#[derive(Debug)]
pub struct SkinnedMotionOutput<'a> {
    pub presentation_id: u64,
    pub reset_history: bool,
    pub motion: &'a wgpu::Texture,
}

#[derive(Debug)]
pub struct Renderer {
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
    pipeline: wgpu::RenderPipeline,
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
    camera: CameraView,
    chunk_meta_buffer: wgpu::Buffer,
    light_buffer: wgpu::Buffer,
    material_layout: wgpu::BindGroupLayout,
    material_pack: MaterialPack,
    material_set: MaterialSet,
    material_bind_group: wgpu::BindGroup,
    quad_buffer: wgpu::Buffer,
    quad_count: u32,
    chunk_draws: Vec<([i32; 4], std::ops::Range<u32>)>,
    resident_camera_anchor: ChunkPos,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
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
    #[allow(clippy::too_many_lines)]
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
    #[allow(clippy::too_many_lines)]
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
    #[allow(clippy::too_many_lines)]
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
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Voxy device"),
                ..Default::default()
            })
            .await
            .map_err(RendererError::RequestDevice)?;
        let device_failure = std::sync::Arc::new(std::sync::OnceLock::new());
        let failure_callback = std::sync::Arc::clone(&device_failure);
        device.set_device_lost_callback(move |reason, message| {
            let _ = failure_callback.set(format!("{reason:?}: {message}"));
        });
        let config = surface
            .get_default_config(&adapter, width.max(1), height.max(1))
            .ok_or(RendererError::UnsupportedSurface)?;
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
        let skin_layout = create_skin_layout(&device);
        let skinned_pipeline = create_skinned_pipeline(
            &device,
            config.format,
            &camera_layout,
            &material_layout,
            &skin_layout,
        );
        let camera = CameraView::default();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Voxy camera uniform"),
            contents: bytemuck::bytes_of(&camera_uniform(config.width, config.height, camera)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let chunk_meta_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Voxy initial chunk metadata"),
            contents: bytemuck::bytes_of(&ChunkMeta::zeroed()),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Voxy initial light data"),
            contents: bytemuck::bytes_of(&u32::from_le_bytes([0xf0; 4])),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let camera_bind_group = create_camera_bind_group(
            &device,
            &camera_layout,
            &camera_buffer,
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
            usage: wgpu::BufferUsages::VERTEX,
        });
        let (depth_texture, depth_view) = create_depth(&device, config.width, config.height);
        let validation_error = validation_scope.pop().await;
        let allocation_error = allocation_scope.pop().await;
        if let Some(error) = validation_error.or(allocation_error) {
            return Err(RendererError::Initialization(error));
        }
        Ok(Self {
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
            camera,
            chunk_meta_buffer,
            light_buffer,
            material_layout,
            material_pack,
            material_set,
            material_bind_group,
            quad_buffer,
            quad_count: 0,
            chunk_draws: Vec::new(),
            resident_camera_anchor: ChunkPos { x: 0, y: 0, z: 0 },
            depth_texture,
            depth_view,
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
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::bytes_of(&camera_uniform(width, height, self.camera)),
        );
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

    fn upload_chunk_data(
        &mut self,
        chunks: &[(ChunkPos, &ChunkMesh, Option<&[u8]>)],
        camera_anchor: ChunkPos,
    ) -> Result<(), RendererError> {
        self.check_device()?;
        if chunks.len() > 1024 {
            return Err(RendererError::TooManyChunks(chunks.len()));
        }
        let mut quads = Vec::new();
        let mut metadata = Vec::with_capacity(chunks.len().max(1));
        let mut light_words = Vec::new();
        let mut chunk_draws = Vec::with_capacity(chunks.len());
        for &(pos, mesh, light) in chunks {
            let relative_origin = relative_chunk_origin(pos, camera_anchor)?;
            let start = u32::try_from(quads.len()).map_err(|_| RendererError::MeshTooLarge)?;
            let slot = metadata.len();
            let slot =
                u16::try_from(slot).map_err(|_| RendererError::TooManyChunks(chunks.len()))?;
            let light_word_offset =
                u32::try_from(light_words.len()).map_err(|_| RendererError::LightDataTooLarge)?;
            pack_light_words(light, &mut light_words)?;
            metadata.push(ChunkMeta {
                relative_origin,
                light_info: [light_word_offset, 0, 0, 0],
            });
            quads.extend(
                mesh.opaque
                    .iter()
                    .chain(mesh.cutout.iter())
                    .chain(mesh.translucent.iter())
                    .copied()
                    .map(|quad| GpuQuad::from(quad).with_chunk_slot(slot)),
            );
            let end = u32::try_from(quads.len()).map_err(|_| RendererError::MeshTooLarge)?;
            chunk_draws.push((relative_origin, start..end));
        }
        let quad_count = u32::try_from(quads.len()).map_err(|_| RendererError::MeshTooLarge)?;
        if metadata.is_empty() {
            metadata.push(ChunkMeta::zeroed());
        }
        if light_words.is_empty() {
            light_words.push(u32::from_le_bytes([0xf0; 4]));
        }
        let metadata_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Voxy chunk metadata"),
                contents: bytemuck::cast_slice(&metadata),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let quad_buffer = if quads.is_empty() {
            None
        } else {
            Some(
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Voxy resident chunk quads"),
                        contents: bytemuck::cast_slice(&quads),
                        usage: wgpu::BufferUsages::VERTEX,
                    }),
            )
        };
        let light_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Voxy resident packed light"),
                contents: bytemuck::cast_slice(&light_words),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let camera_bind_group = create_camera_bind_group(
            &self.device,
            &self.camera_layout,
            &self.camera_buffer,
            &metadata_buffer,
            &light_buffer,
        );
        self.chunk_meta_buffer = metadata_buffer;
        self.light_buffer = light_buffer;
        self.camera_bind_group = camera_bind_group;
        if let Some(buffer) = quad_buffer {
            self.quad_buffer = buffer;
        }
        self.quad_count = quad_count;
        self.chunk_draws = chunk_draws;
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
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::bytes_of(&camera_uniform(
                self.config.width,
                self.config.height,
                camera,
            )),
        );
        Ok(())
    }

    /// Replaces the single currently resident animated mesh and joint palette.
    ///
    /// # Errors
    ///
    /// Rejects palette mismatch, non-finite transforms, or an excessive index count.
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
    /// Rejects missing geometry, palette length mismatch, or non-finite matrices.
    pub fn update_skin_matrices(&mut self, joints: &[Mat4]) -> Result<(), RendererError> {
        self.check_device()?;
        let lod_pose = self
            .skinned_lod
            .as_ref()
            .map(|lod| {
                lod.pose
                    .source()
                    .prepare(joints, lod.pose.model())
                    .map_err(|error| RendererError::SkinnedLod(crate::SkinnedLodGpuError::Pose(error)))
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
        self.queue
            .write_buffer(&skinned.joint_buffer, 0, bytemuck::cast_slice(joints));
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

    /// Updates the animated object's world transform without rebuilding its mesh.
    ///
    /// # Errors
    ///
    /// Rejects a missing skinned mesh or non-finite transform.
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
                    .map_err(|error| RendererError::SkinnedLod(crate::SkinnedLodGpuError::Pose(error)))
            })
            .transpose()?;
        let skinned = self.skinned.as_ref().ok_or(RendererError::NoSkinnedMesh)?;
        let uniform = crate::skinned::object_uniform(model, skinned.material_layer);
        self.queue
            .write_buffer(&skinned.object_buffer, 0, bytemuck::bytes_of(&uniform));
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
        if self.state == SurfaceState::Suspended {
            return Ok(RenderOutcome::Suspended);
        }
        let frame_id = self
            .resident_presented_frames
            .checked_add(1)
            .ok_or(RendererError::FrameIdExhausted)?;
        let (frame, suboptimal) = match self.surface.get_current_texture() {
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
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Voxy voxel pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
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
            if self.quad_count != 0 {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_bind_group(1, &self.material_bind_group, &[]);
                pass.set_vertex_buffer(0, self.quad_buffer.slice(..));
                let view_projection =
                    camera_view_projection(self.config.width, self.config.height, self.camera);
                for (origin, instances) in &self.chunk_draws {
                    if chunk_intersects_frustum(view_projection, *origin) {
                        pass.draw(0..6, instances.clone());
                    }
                }
            }
            if let Some(skinned) = &self.skinned {
                pass.set_pipeline(&self.skinned_pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_bind_group(1, &self.material_bind_group, &[]);
                pass.set_bind_group(2, &skinned.bind_group, &[]);
                pass.set_vertex_buffer(0, skinned.vertex.slice(..));
                pass.set_index_buffer(skinned.index.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..skinned.index_count, 0, 0..1);
            }
        }
        encode_motion_layers(&mut encoder, motion_pass.as_ref(), base_motion.as_ref())?;
        self.queue.submit([encoder.finish()]);
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

    /// Creates the general scene pipelines for this surface's color format.
    #[must_use]
    pub fn create_scene_renderer(&self) -> crate::SceneRenderer {
        crate::SceneRenderer::new(&self.device, self.config.format)
    }

    /// Presents a general scene using this renderer's surface lifecycle.
    /// Scene resources must have been created on this device.
    ///
    /// # Errors
    /// Returns surface/device-loss errors requiring the platform shell to recreate graphics.
    pub fn render_scene(
        &mut self,
        scene: &crate::SceneRenderer,
        draws: &[crate::SceneDraw<'_>],
    ) -> Result<RenderOutcome, RendererError> {
        self.check_device()?;
        self.reset_skinned_motion();
        if self.state == SurfaceState::Suspended {
            return Ok(RenderOutcome::Suspended);
        }
        let (frame, suboptimal) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(RenderOutcome::Reconfigured);
            }
            wgpu::CurrentSurfaceTexture::Lost => return Err(RendererError::SurfaceLost),
            wgpu::CurrentSurfaceTexture::Timeout => return Ok(RenderOutcome::SkippedTimeout),
            wgpu::CurrentSurfaceTexture::Occluded => return Ok(RenderOutcome::SkippedOccluded),
            wgpu::CurrentSurfaceTexture::Validation => {
                return Ok(RenderOutcome::SkippedValidation);
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Voxy frame encoder"),
            });
        scene.encode(
            &mut encoder,
            &view,
            &self.depth_view,
            self.clear_color,
            draws,
        );
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        if suboptimal {
            self.surface.configure(&self.device, &self.config);
            Ok(RenderOutcome::Reconfigured)
        } else {
            Ok(RenderOutcome::Presented)
        }
    }

    #[must_use]
    pub const fn surface_state(&self) -> SurfaceState {
        self.state
    }

    #[must_use]
    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.adapter.get_info()
    }

    /// Shared managed compute-storage budget retained by this device owner.
    #[must_use]
    pub fn compute_memory_budget(&self) -> &crate::ComputeMemoryBudget {
        &self.compute_memory_budget
    }

    /// Shared staging pool retained for the lifetime of this renderer.
    #[must_use]
    pub fn readback_pool(&self) -> &crate::ComputeReadbackPool {
        &self.readback_pool
    }

    #[must_use]
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    #[must_use]
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}

fn create_camera_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Voxy camera layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

fn create_camera_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    camera: &wgpu::Buffer,
    chunk_metadata: &wgpu::Buffer,
    light_data: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Voxy frame data bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: chunk_metadata.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: light_data.as_entire_binding(),
            },
        ],
    })
}

fn create_material_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Voxy material layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

fn create_material_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    materials: &MaterialSet,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Voxy material bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&materials.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&materials.sampler),
            },
        ],
    })
}

fn create_pipeline(
    device: &wgpu::Device,
    color_format: wgpu::TextureFormat,
    camera_layout: &wgpu::BindGroupLayout,
    material_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Voxy voxel shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("voxel.wgsl").into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Voxy voxel pipeline layout"),
        bind_group_layouts: &[Some(camera_layout), Some(material_layout)],
        immediate_size: 0,
    });
    let attributes = wgpu::vertex_attr_array![
        0 => Uint8x4,
        1 => Uint8x4,
        2 => Uint16x2,
        3 => Uint8x4,
        4 => Uint16x2
    ];
    let vertex_buffer = wgpu::VertexBufferLayout {
        array_stride: size_of::<GpuQuad>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &attributes,
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Voxy voxel pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(vertex_buffer)],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn create_depth(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Voxy depth texture"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

#[allow(clippy::cast_precision_loss)]
fn camera_uniform(width: u32, height: u32, camera: CameraView) -> CameraUniform {
    CameraUniform {
        view_proj: camera_view_projection(width, height, camera).to_cols_array_2d(),
    }
}

#[allow(clippy::cast_precision_loss)]
fn camera_view_projection(width: u32, height: u32, camera: CameraView) -> Mat4 {
    let [half_width, half_height] = camera_half_extents(width, height, camera);
    // Swap near/far to retain the renderer's reverse-Z depth convention.
    let projection = orthographic(
        -half_width,
        half_width,
        -half_height,
        half_height,
        512.0,
        camera.near_plane,
    );
    let view = look_at_mat4(camera.eye, camera.target, camera.up);
    projection * view
}

#[allow(clippy::cast_precision_loss)]
fn camera_half_extents(width: u32, height: u32, camera: CameraView) -> [f32; 2] {
    let aspect = width.max(1) as f32 / height.max(1) as f32;
    let distance = camera.eye.distance(camera.target);
    let half_height = (distance * (camera.vertical_fov_radians * 0.5).tan()).max(1.0);
    let half_width = half_height * aspect;
    [half_width, half_height]
}

fn chunk_intersects_frustum(view_projection: Mat4, relative_origin: [i32; 4]) -> bool {
    #[allow(clippy::cast_precision_loss)]
    let min = Vec3::new(
        relative_origin[0] as f32,
        relative_origin[1] as f32,
        relative_origin[2] as f32,
    );
    #[allow(clippy::cast_precision_loss)]
    let max = min + Vec3::splat(CHUNK_EDGE as f32);
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(max.x, max.y, max.z),
    ]
    .map(|corner| view_projection * corner.extend(1.0));
    let planes: [fn(Vec4) -> f32; 6] = [
        |point| point.x + point.w,
        |point| point.w - point.x,
        |point| point.y + point.w,
        |point| point.w - point.y,
        |point| point.z,
        |point| point.w - point.z,
    ];
    planes
        .into_iter()
        .all(|plane| corners.iter().any(|&corner| plane(corner) >= 0.0))
}

fn default_material_pack() -> MaterialPack {
    let colors: [[u8; 4]; 5] = [
        [48, 65, 42, 255],
        [90, 105, 66, 255],
        [132, 145, 87, 255],
        [76, 91, 57, 255],
        [151, 161, 99, 120],
    ];
    let layers = colors
        .into_iter()
        .map(|color| {
            let mut bytes = Vec::with_capacity(16 * 16 * 4);
            for y in 0..16 {
                for x in 0..16 {
                    bytes.extend(color.map(|channel| {
                        if channel == 255 || (x / 4 + y / 4) % 2 == 0 {
                            channel
                        } else {
                            u8::try_from(u16::from(channel) * 82 / 100).unwrap_or(channel)
                        }
                    }));
                }
            }
            MaterialLayer {
                rgba8_srgb: Arc::from(bytes),
            }
        })
        .collect();
    MaterialPack::new(16, 16, layers).expect("built-in material pack is valid")
}

fn relative_chunk_origin(pos: ChunkPos, anchor: ChunkPos) -> Result<[i32; 4], RendererError> {
    fn component(value: i64, anchor: i64) -> Result<i32, RendererError> {
        value
            .checked_sub(anchor)
            .and_then(|delta| delta.checked_mul(CHUNK_EDGE))
            .and_then(|voxels| i32::try_from(voxels).ok())
            .ok_or(RendererError::OriginOutOfRange)
    }
    Ok([
        component(pos.x, anchor.x)?,
        component(pos.y, anchor.y)?,
        component(pos.z, anchor.z)?,
        0,
    ])
}

fn pack_light_words(light: Option<&[u8]>, output: &mut Vec<u32>) -> Result<(), RendererError> {
    let bytes = light.unwrap_or(&[]);
    if !bytes.is_empty() && bytes.len() != voxy_core::CHUNK_VOLUME {
        return Err(RendererError::InvalidLightVolume(bytes.len()));
    }
    if bytes.is_empty() {
        output.extend(std::iter::repeat_n(
            u32::from_le_bytes([0xf0; 4]),
            voxy_core::CHUNK_VOLUME / 4,
        ));
    } else {
        output.extend(
            bytes
                .chunks_exact(4)
                .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])),
        );
    }
    Ok(())
}

#[derive(Debug)]
pub enum RendererError {
    InvalidPrefilterSamples(u32),
    /// Shader, pipeline, surface configuration or allocation failed before use.
    Initialization(wgpu::Error),
    InvalidSurfaceSize {
        width: u32,
        height: u32,
        max_dimension: u32,
    },
    CreateSurface(wgpu::CreateSurfaceError),
    RequestAdapter(wgpu::RequestAdapterError),
    RequestDevice(wgpu::RequestDeviceError),
    UnsupportedSurface,
    FrameIdExhausted,
    /// A native/temporal consumer failed after scene submission, before present.
    TemporalConsumer(String),
    SurfaceLost,
    DeviceLost(String),
    MeshTooLarge,
    TooManyChunks(usize),
    OriginOutOfRange,
    InvalidLightVolume(usize),
    LightDataTooLarge,
    Skinned(SkinnedUploadError),
    SkinnedLod(crate::SkinnedLodGpuError),
    Ray(crate::RaySceneError),
    NoSkinnedMesh,
    NoSkinnedLod,
    InvalidCamera,
    Scene(crate::SceneError),
}

impl fmt::Display for RendererError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "renderer error: {self:?}")
    }
}

impl std::error::Error for RendererError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CreateSurface(error) => Some(error),
            Self::RequestAdapter(error) => Some(error),
            Self::RequestDevice(error) => Some(error),
            Self::Skinned(error) => Some(error),
            Self::SkinnedLod(error) => Some(error),
            Self::Ray(error) => Some(error),
            Self::Scene(error) => Some(error),
            _ => None,
        }
    }
}

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

    #[test]
    fn clip_space_aabb_culling_rejects_fully_outside_chunks() {
        assert!(chunk_intersects_frustum(Mat4::IDENTITY, [0, 0, 0, 0]));
        assert!(!chunk_intersects_frustum(Mat4::IDENTITY, [2, 0, 0, 0]));
        assert!(!chunk_intersects_frustum(Mat4::IDENTITY, [0, 0, -33, 0]));
    }

    #[test]
    #[allow(clippy::too_many_lines)]
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
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &camera_bind, &[]);
            pass.set_bind_group(1, &material_bind, &[]);
            pass.set_vertex_buffer(0, quads.slice(..));
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
}

fn encode_motion_layers(
    encoder: &mut wgpu::CommandEncoder,
    motion: Option<&crate::RasterMotionPass>,
    base: Option<&crate::DepthMotionPass>,
) -> Result<(), RendererError> {
    if let (Some(motion), Some(base)) = (motion, base) {
        base.encode(encoder);
        motion
            .encode_over(encoder, base.output())
            .map_err(RendererError::Ray)?;
    }
    Ok(())
}
