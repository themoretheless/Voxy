//! General textured triangle rendering, independent of voxel storage.

/// Default shader and the resource/vertex ABI for custom scene shaders.
pub const DEFAULT_SCENE_SHADER: &str = include_str!("scene.wgsl");

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TextureWrap {
    #[default]
    Clamp,
    Repeat,
    Mirror,
}
impl TextureWrap {
    fn address_mode(self) -> wgpu::AddressMode {
        match self {
            Self::Clamp => wgpu::AddressMode::ClampToEdge,
            Self::Repeat => wgpu::AddressMode::Repeat,
            Self::Mirror => wgpu::AddressMode::MirrorRepeat,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TextureFilter {
    #[default]
    Nearest,
    Linear,
}
impl TextureFilter {
    fn filter_mode(self) -> wgpu::FilterMode {
        match self {
            Self::Nearest => wgpu::FilterMode::Nearest,
            Self::Linear => wgpu::FilterMode::Linear,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextureSampling {
    pub wrap_u: TextureWrap,
    pub wrap_v: TextureWrap,
    pub min_filter: TextureFilter,
    pub mag_filter: TextureFilter,
    /// Independent mip-level interpolation; None preserves the legacy min-filter choice.
    pub mipmap_filter: Option<TextureFilter>,
    /// Requested anisotropy, 1 (disabled) through 16. Values above 1 require
    /// linear minification, magnification and mip interpolation. Downlevel backends may use 1.
    pub anisotropy: u16,
}
impl Default for TextureSampling {
    fn default() -> Self {
        Self {
            wrap_u: TextureWrap::default(),
            wrap_v: TextureWrap::default(),
            min_filter: TextureFilter::default(),
            mag_filter: TextureFilter::default(),
            mipmap_filter: None,
            anisotropy: 1,
        }
    }
}
impl TextureSampling {
    fn descriptor(self) -> wgpu::SamplerDescriptor<'static> {
        wgpu::SamplerDescriptor {
            anisotropy_clamp: self.anisotropy,
            address_mode_u: self.wrap_u.address_mode(),
            address_mode_v: self.wrap_v.address_mode(),
            min_filter: self.min_filter.filter_mode(),
            mag_filter: self.mag_filter.filter_mode(),
            mipmap_filter: match self.mipmap_filter.unwrap_or(self.min_filter) {
                TextureFilter::Nearest => wgpu::MipmapFilterMode::Nearest,
                TextureFilter::Linear => wgpu::MipmapFilterMode::Linear,
            },
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct SceneShaderError(pub String);
impl std::fmt::Display for SceneShaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for SceneShaderError {}
use glam::Mat4;

mod geometry;
pub(crate) use geometry::{NormalCache, smooth_normals};
pub use geometry::{SceneError, SceneMesh, SceneVertex};

fn scene_memory_error(error: crate::ComputeError) -> SceneError {
    match error {
        crate::ComputeError::MemoryBudget => SceneError::MemoryBudget,
        _ => SceneError::InvalidGeometry,
    }
}
fn managed_scene_indices(
    device: &wgpu::Device,
    indices: &[u32],
) -> Result<std::sync::Arc<crate::ComputeStorage>, SceneError> {
    let [buffer] = crate::ComputeMemoryBudget::for_device(device)
        .allocate_buffers([voxy_gpu_core::ManagedBufferDescriptor {
            label: "scene LOD indices",
            size: indices.len() as u64 * 4,
            contents: Some(bytemuck::cast_slice(indices)),
            usage: wgpu::BufferUsages::INDEX,
        }])
        .map_err(scene_memory_error)?;
    Ok(std::sync::Arc::new(buffer))
}
/// Depth policy for ordinary geometry, ghost shells and revealed internal layers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SceneDepthMode {
    #[default]
    Opaque,
    /// Tests world depth without writing it. Supply triangles back to front.
    Transparent,
    /// Reveals selected geometry through occluders without modifying world depth.
    Xray,
}

#[derive(Debug)]
pub struct SceneGeometry {
    device: wgpu::Device,
    vertices: std::sync::Arc<crate::ComputeStorage>,
    normals: std::sync::Arc<crate::ComputeStorage>,
    normal_cache: NormalCache,
    material_coordinates: std::sync::Arc<crate::ComputeStorage>,
    coordinate_cache: Vec<[f32; 3]>,
    material_parameters: std::sync::Arc<crate::ComputeStorage>,
    indices: std::sync::Arc<crate::ComputeStorage>,
    index_count: u32,
    vertex_capacity: usize,
    index_capacity: usize,
    depth_mode: SceneDepthMode,
    opaque_shader: Option<(
        u64,
        wgpu::BindGroupLayout,
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
    )>,
    partitioned_indices: bool,
    partition_cache: Vec<u32>,
    /// Process-unique identity so shadow consumers can recognise the same allocation.
    identity: u64,
    /// Bumped on every GPU write of vertices or indices; part of the shadow key.
    revision: u64,
}

static NEXT_GEOMETRY_IDENTITY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub(crate) fn next_geometry_identity() -> u64 {
    NEXT_GEOMETRY_IDENTITY.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// Additional resource owners publish with their geometry after one admission.
struct UploadedGeometryBatch {
    geometries: Vec<SceneGeometry>,
    additional_buffers: Vec<std::sync::Arc<crate::ComputeStorage>>,
}

impl SceneGeometry {
    pub(crate) fn deformation_vertex_capacity(&self) -> usize {
        self.vertex_capacity
    }
    pub(crate) fn deformation_normals(&self) -> &wgpu::Buffer {
        &self.normals
    }
    pub(crate) fn deformation_vertices(&self) -> &wgpu::Buffer {
        &self.vertices
    }
    pub(crate) fn belongs_to(&self, device: &wgpu::Device) -> bool {
        &self.device == device
    }
    pub(crate) fn encode_shadow_geometry(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.index_count, 0, 0..1);
    }

    /// Identity and revision of the geometry's current GPU contents. Two equal keys
    /// mean the same allocation with no vertex/index writes in between.
    #[must_use]
    pub(crate) fn shadow_key(&self) -> (u64, u64) {
        (self.identity, self.revision)
    }

    /// Records a GPU write of vertex or index data.
    pub(crate) fn mark_shadow_dirty(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn set_depth_mode(&mut self, mode: SceneDepthMode) {
        self.depth_mode = mode;
    }
    #[must_use]
    pub fn depth_mode(&self) -> SceneDepthMode {
        self.depth_mode
    }

    /// Updates an existing allocation. Draws encoded afterward use the new index count.
    /// Submit already encoded draws before updating their geometry.
    /// # Errors
    /// Rejects invalid meshes and capacity overflow before writing either buffer.
    pub fn update(&mut self, queue: &wgpu::Queue, mesh: &SceneMesh) -> Result<(), SceneError> {
        self.update_mesh(queue, mesh, false)
    }

    /// Updates shared vertex streams without restoring the full index stream when
    /// the mesh topology is unchanged. A topology change restores the full mesh;
    /// callers must then install their new material partitions before drawing.
    /// Submit prior draws before updating the shared allocation.
    pub fn update_shared_vertex_streams(
        &mut self,
        queue: &wgpu::Queue,
        mesh: &SceneMesh,
    ) -> Result<(), SceneError> {
        self.update_mesh(queue, mesh, true)
    }

    fn update_mesh(
        &mut self,
        queue: &wgpu::Queue,
        mesh: &SceneMesh,
        preserve_partition: bool,
    ) -> Result<(), SceneError> {
        mesh.validate_for_upload()?;
        if mesh.vertices.len() > self.vertex_capacity || mesh.indices.len() > self.index_capacity {
            return Err(SceneError::GeometryCapacityExceeded);
        }
        self.mark_shadow_dirty();

        // One index comparison serves partition retention, the index upload
        // decision and the normal cache refresh below.
        let indices_equal = self.normal_cache.indices == mesh.indices;
        let retain_partition = preserve_partition
            && self.partitioned_indices
            && indices_equal
            && self.normal_cache.positions.len() == mesh.vertices.len();
        let indices_changed = !retain_partition && (self.partitioned_indices || !indices_equal);
        let coordinates = mesh.material_coordinates();
        if coordinates.as_ref() != self.coordinate_cache.as_slice() {
            queue.write_buffer(
                &self.material_coordinates,
                0,
                bytemuck::cast_slice(coordinates.as_ref()),
            );
            self.coordinate_cache = coordinates.into_owned();
        }
        let count = u32::try_from(mesh.indices.len()).map_err(|_| SceneError::InvalidGeometry)?;
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&mesh.vertices));
        if self.normal_cache.refresh_with(mesh, indices_equal) {
            queue.write_buffer(
                &self.normals,
                0,
                bytemuck::cast_slice(&self.normal_cache.normals),
            );
        }
        if indices_changed {
            queue.write_buffer(&self.indices, 0, bytemuck::cast_slice(&mesh.indices));
        }
        queue.write_buffer(
            &self.material_parameters,
            0,
            bytemuck::cast_slice(&mesh.material_parameters),
        );
        if !retain_partition {
            self.index_count = count;
            self.partitioned_indices = false;
            self.partition_cache.clear();
        }
        Ok(())
    }

    #[must_use]
    pub fn update_index_partition(
        &mut self,
        queue: &wgpu::Queue,
        indices: &[u32],
    ) -> Result<(), SceneError> {
        self.update_index_partition_if_changed(queue, indices)
            .map(|_| ())
    }

    /// Returns whether the index allocation changed. Exact validated cache hits
    /// retain the existing GPU stream; full-mesh updates invalidate this cache.
    pub fn update_index_partition_if_changed(
        &mut self,
        queue: &wgpu::Queue,
        indices: &[u32],
    ) -> Result<bool, SceneError> {
        if self.partitioned_indices && self.partition_cache == indices {
            return Ok(false);
        }
        if indices.len() > self.index_capacity
            || !indices.len().is_multiple_of(3)
            || indices.iter().any(|i| *i as usize >= self.vertex_capacity)
        {
            return Err(SceneError::InvalidGeometry);
        }
        let count = u32::try_from(indices.len()).map_err(|_| SceneError::InvalidGeometry)?;
        if !indices.is_empty() {
            queue.write_buffer(&self.indices, 0, bytemuck::cast_slice(indices));
        }
        self.mark_shadow_dirty();
        self.partition_cache.clear();
        self.partition_cache.extend_from_slice(indices);
        self.index_count = count;
        self.partitioned_indices = true;
        Ok(true)
    }
    #[must_use]
    pub fn index_count(&self) -> u32 {
        self.index_count
    }

    /// Logical bytes in this geometry's five GPU buffers, including aligned
    /// allocations and retained capacity. Excludes driver and CPU cache storage.
    #[must_use]
    pub fn allocation_bytes(&self) -> u64 {
        self.vertices.size()
            + self.indices.size()
            + self.normals.size()
            + self.material_coordinates.size()
            + self.material_parameters.size()
    }

    /// Stops drawing while retaining the allocation for later updates.
    pub fn clear(&mut self) {
        self.index_count = 0;
    }

    #[must_use]
    pub fn capacity(&self) -> (usize, usize) {
        (self.vertex_capacity, self.index_capacity)
    }
}

fn geometry_sizes(
    device: &wgpu::Device,
    vertices: usize,
    indices: usize,
) -> Result<(u64, u64), SceneError> {
    let vertex_bytes = u64::try_from(vertices)
        .ok()
        .and_then(|n| n.checked_mul(std::mem::size_of::<SceneVertex>() as u64));
    let index_bytes = u64::try_from(indices).ok().and_then(|n| n.checked_mul(4));
    let limit = device.limits().max_buffer_size;
    match (vertex_bytes, index_bytes) {
        (Some(v), Some(i))
            if v > 0
                && i > 0
                && v <= limit
                && i <= limit
                && u32::try_from(vertices).is_ok()
                && u32::try_from(indices).is_ok() =>
        {
            Ok((v, i))
        }
        _ => Err(SceneError::GeometryCapacityExceeded),
    }
}

#[derive(Debug)]
pub struct SceneTexture {
    device: wgpu::Device,
    bind_group: wgpu::BindGroup,
    texture: std::sync::Arc<voxy_gpu_core::ManagedTexture>,
    sampler: wgpu::Sampler,
}
impl SceneTexture {
    pub(crate) fn belongs_to(&self, device: &wgpu::Device) -> bool {
        &self.device == device
    }
    /// Reuse the uploaded sRGB image and mip levels in other passes on this device.
    #[must_use]
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
    /// Logical payload across all allocated mip levels, shared by material views.
    #[must_use]
    pub fn allocation_bytes(&self) -> u64 {
        self.texture.allocation_bytes()
    }
    /// Reuse the material's wrapping/filtering/anisotropy configuration.
    #[must_use]
    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }
}

/// A draw owns its uniform so updating one object cannot overwrite another draw.
#[derive(Debug)]
pub struct SceneTransform {
    buffer: crate::ComputeStorage,
    bind_group: wgpu::BindGroup,
}
impl SceneTransform {
    /// Logical bytes retained by this transform in the shared device ledger.
    #[must_use]
    pub fn allocation_bytes(&self) -> u64 {
        self.buffer.size()
    }
    /// Set the positive world half-space for `planar_reflection_pbr_clip_shader`.
    /// Preserves world/tint/light/view/PBR. Reapply after MVP or motion updates,
    /// which overwrite this plane in the previous-MVP slot.
    /// # Errors
    /// Rejects nonfinite planes and zero normals before writing GPU memory.
    pub fn update_pbr_capture_plane(
        &self,
        queue: &wgpu::Queue,
        point: glam::Vec3,
        normal: glam::Vec3,
    ) -> Result<(), SceneError> {
        let normal = normal.try_normalize().ok_or(SceneError::InvalidTransform)?;
        let offset = -normal.dot(point);
        if !point.is_finite() || !offset.is_finite() {
            return Err(SceneError::InvalidTransform);
        }
        queue.write_buffer(
            &self.buffer,
            64,
            bytemuck::cast_slice(&[normal.x, normal.y, normal.z, offset]),
        );
        Ok(())
    }
    /// Set world transform, tint and clipping plane for `PLANAR_REFLECTION_CLIP_SHADER`.
    /// Keep the positive half-space, including the plane. Uses the point-light slot;
    /// this is a dedicated unlit capture material, not a point-light shader setting.
    /// # Errors
    /// Rejects invalid transforms/tint, nonfinite planes or zero normals before writing.
    pub fn update_planar_clip(
        &self,
        queue: &wgpu::Queue,
        world: Mat4,
        tint: [f32; 4],
        plane_point: glam::Vec3,
        plane_normal: glam::Vec3,
    ) -> Result<(), SceneError> {
        let normal = plane_normal
            .try_normalize()
            .ok_or(SceneError::InvalidTransform)?;
        let offset = -normal.dot(plane_point);
        if !world.is_finite()
            || !tint.into_iter().all(f32::is_finite)
            || !plane_point.is_finite()
            || !offset.is_finite()
        {
            return Err(SceneError::InvalidTransform);
        }
        let mut values = [0.; 24];
        values[..16].copy_from_slice(&world.to_cols_array());
        values[16..20].copy_from_slice(&tint);
        values[20..24].copy_from_slice(&[normal.x, normal.y, normal.z, offset]);
        queue.write_buffer(&self.buffer, 144, bytemuck::cast_slice(&values));
        Ok(())
    }
    /// Set display MVP and reflected-camera MVP for `PLANAR_REFLECTION_SURFACE_SHADER`.
    /// Both matrices include this mesh's world transform. The second matrix replaces
    /// motion history; use a separate transform for a motion-vector pass.
    /// # Errors
    /// Rejects nonfinite matrices before any GPU write.
    pub fn update_planar_projection(
        &self,
        queue: &wgpu::Queue,
        display_mvp: Mat4,
        capture_mvp: Mat4,
    ) -> Result<(), SceneError> {
        self.update_motion(
            queue,
            crate::MotionMatrices {
                current: display_mvp,
                previous: capture_mvp,
                history_valid: true,
            },
        )
    }
    /// # Errors
    /// Rejects non-finite matrices before writing GPU memory.
    pub fn update(&self, queue: &wgpu::Queue, mvp: Mat4) -> Result<(), SceneError> {
        self.update_motion(
            queue,
            crate::MotionMatrices {
                current: mvp,
                previous: mvp,
                history_valid: false,
            },
        )
    }

    /// Updates optional editor material data after the existing camera ABI prefix.
    /// Legacy shaders keep the first 144 bytes and ignore these fields.
    /// Existing PBR parameters remain unchanged when world/tint/light are updated.
    /// # Errors
    /// Rejects non-finite transforms/colors and invalid light intensity.
    pub fn update_scene_material(
        &self,
        queue: &wgpu::Queue,
        world: Mat4,
        tint: [f32; 4],
        light: [f32; 4],
    ) -> Result<(), SceneError> {
        if !world.is_finite()
            || !tint.into_iter().all(f32::is_finite)
            || !light.into_iter().all(f32::is_finite)
            || light[3] < 0.0
        {
            return Err(SceneError::InvalidTransform);
        }
        let mut values = [0.; 25];
        values[..16].copy_from_slice(&world.to_cols_array());
        values[16..20].copy_from_slice(&tint);
        values[20..24].copy_from_slice(&light);
        values[24] = 1.; // authored opaque material; overlays keep the default zero
        queue.write_buffer(&self.buffer, 144, bytemuck::cast_slice(&values));
        Ok(())
    }

    /// Sets GGX roughness and metallic weight for this draw.
    /// World/tint/light updates preserve these settings.
    /// # Errors
    /// Rejects non-finite values and values outside [0, 1] before writing GPU memory.
    pub fn update_pbr_material(
        &self,
        queue: &wgpu::Queue,
        roughness: f32,
        metallic: f32,
    ) -> Result<(), SceneError> {
        if !roughness.is_finite()
            || !metallic.is_finite()
            || !(0.0..=1.0).contains(&roughness)
            || !(0.0..=1.0).contains(&metallic)
        {
            return Err(SceneError::InvalidTransform);
        }
        // authored.yzw: roughness, metallic, explicit PBR marker.
        queue.write_buffer(
            &self.buffer,
            244,
            bytemuck::cast_slice(&[roughness, metallic, 1.0]),
        );
        Ok(())
    }

    /// Sets the camera position in the mesh's coordinate system for material shaders.
    /// Matrix updates preserve this field; older shaders may use the 128-byte prefix.
    /// # Errors
    /// Rejects non-finite positions before writing GPU memory.
    pub fn update_view_position(
        &self,
        queue: &wgpu::Queue,
        position: glam::Vec3,
    ) -> Result<(), SceneError> {
        if !position.is_finite() {
            return Err(SceneError::InvalidTransform);
        }
        queue.write_buffer(
            &self.buffer,
            128,
            bytemuck::cast_slice(&[position.x, position.y, position.z, 1.]),
        );
        Ok(())
    }
    /// Uploads current/previous MVPs for temporal shaders. Invalid history uses zero motion.
    /// # Errors
    /// Rejects non-finite matrices before any GPU write.
    pub fn update_motion(
        &self,
        queue: &wgpu::Queue,
        motion: crate::MotionMatrices,
    ) -> Result<(), SceneError> {
        if !motion.current.is_finite() || !motion.previous.is_finite() {
            return Err(SceneError::InvalidTransform);
        }
        let previous = if motion.history_valid {
            motion.previous
        } else {
            motion.current
        };
        queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::cast_slice(&[motion.current, previous]),
        );
        Ok(())
    }
}

#[derive(Debug)]
pub struct SceneDraw<'a> {
    pub geometry: &'a SceneGeometry,
    pub texture: &'a SceneTexture,
    pub transform: &'a SceneTransform,
    /// Draw after world geometry in painter order; ignores and does not write depth.
    pub overlay: bool,
}

/// A disjoint physical-pixel viewport in a shared render target. Draw transforms
/// contain this view's projection; model geometry and textures may be shared.
#[derive(Debug)]
pub struct SceneView<'a> {
    /// [left, top, width, height], matching the view's LOD selection dimensions.
    pub viewport: [u32; 4],
    pub draws: &'a [SceneDraw<'a>],
}

/// Borrowed attachments for one composed view frame. Four-sample color/depth
/// require a single-sample resolve output and overlay depth. X-ray depth, when
/// supplied, must be distinct from world depth and match its sample count/size.
#[derive(Debug)]
pub struct SceneViewTargets<'a> {
    pub color: &'a wgpu::TextureView,
    pub depth: &'a wgpu::TextureView,
    pub xray_depth: Option<&'a wgpu::TextureView>,
    pub resolve: Option<&'a wgpu::TextureView>,
    pub overlay_depth: &'a wgpu::TextureView,
}

struct ViewPass<'a> {
    resolve: Option<&'a wgpu::TextureView>,
    color_load: wgpu::LoadOp<wgpu::Color>,
    depth_load: wgpu::LoadOp<f32>,
    pipelines: (
        &'a wgpu::RenderPipeline,
        &'a wgpu::RenderPipeline,
        &'a wgpu::RenderPipeline,
    ),
    stages: std::ops::Range<u8>,
    isolated_xray: bool,
}

#[derive(Debug)]
pub struct SceneRenderer {
    device: wgpu::Device,
    compute_memory_budget: crate::ComputeMemoryBudget,
    world_pipeline: wgpu::RenderPipeline,
    overlay_pipeline: wgpu::RenderPipeline,
    transparent_pipeline: wgpu::RenderPipeline,
    transform_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    color_format: wgpu::TextureFormat,
    shader_source: String,
    shader_revision: u64,
    shadow: Option<crate::shadow_visibility::ShadowBindings>,
    environment: Option<crate::environment_lighting::EnvironmentBindings>,
    msaa_pipelines: Option<(
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
    )>,
}

impl SceneRenderer {
    /// Shared device owner retained even when no scene resource is resident.
    #[must_use]
    pub fn compute_memory_budget(&self) -> &crate::ComputeMemoryBudget {
        &self.compute_memory_budget
    }
    fn resource_device(&self, supplied: &wgpu::Device) -> Result<&wgpu::Device, SceneError> {
        if supplied != &self.device {
            return Err(SceneError::DeviceMismatch);
        }
        Ok(&self.device)
    }
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let transform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene transform"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(256),
                },
                count: None,
            }],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene texture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
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
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene shader"),
            source: wgpu::ShaderSource::Wgsl(DEFAULT_SCENE_SHADER.into()),
        });
        // Build initial pipelines before constructing the owner.
        let mut renderer =
            Self::from_layouts(device, format, transform_layout, texture_layout, &shader);
        DEFAULT_SCENE_SHADER.clone_into(&mut renderer.shader_source);
        renderer
    }

    /// Creates both single-sample and four-sample presentation pipelines.
    /// The caller must verify four-sample color/depth format support.
    #[must_use]
    pub fn new_msaa4(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let mut renderer = Self::new(device, format);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("MSAA scene shader"),
            source: wgpu::ShaderSource::Wgsl(DEFAULT_SCENE_SHADER.into()),
        });
        renderer.msaa_pipelines = Some(Self::create_shader_pipelines(
            device,
            format,
            &renderer.transform_layout,
            &renderer.texture_layout,
            &shader,
            4,
            None,
            None,
        ));
        renderer
    }

    /// Adds four-sample pipelines to the current shader/layouts without replacing
    /// existing resources. Repeated enable is a no-op. The caller verifies target
    /// format support before configuring a multisampled surface.
    /// # Errors
    /// A foreign device or validation failure leaves existing pipelines intact.
    pub async fn enable_msaa4(&mut self, device: &wgpu::Device) -> Result<bool, SceneShaderError> {
        if device != &self.device {
            return Err(SceneShaderError("MSAA enable device mismatch".into()));
        }
        if self.msaa_pipelines.is_some() {
            return Ok(false);
        }
        let device = &self.device;
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("current scene shader for MSAA"),
            source: wgpu::ShaderSource::Wgsl(self.shader_source.as_str().into()),
        });
        if let Some(error) = scope.pop().await {
            return Err(SceneShaderError(error.to_string()));
        }
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let pipelines = Self::create_shader_pipelines(
            device,
            self.color_format,
            &self.transform_layout,
            &self.texture_layout,
            &shader,
            4,
            self.shadow.as_ref().map(|shadow| &shadow.layout),
            self.environment
                .as_ref()
                .map(|environment| &environment.layout),
        );
        if let Some(error) = scope.pop().await {
            return Err(SceneShaderError(error.to_string()));
        }
        self.msaa_pipelines = Some(pipelines);
        Ok(true)
    }

    fn from_layouts(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        transform_layout: wgpu::BindGroupLayout,
        texture_layout: wgpu::BindGroupLayout,
        shader: &wgpu::ShaderModule,
    ) -> Self {
        let (world_pipeline, overlay_pipeline, transparent_pipeline) =
            Self::create_shader_pipelines(
                device,
                format,
                &transform_layout,
                &texture_layout,
                shader,
                1,
                None,
                None,
            );
        Self {
            device: device.clone(),
            compute_memory_budget: crate::ComputeMemoryBudget::for_device(device),
            world_pipeline,
            overlay_pipeline,
            transparent_pipeline,
            transform_layout,
            texture_layout,
            color_format: format,
            shader_source: String::new(),
            shader_revision: 0,
            shadow: None,
            environment: None,
            msaa_pipelines: None,
        }
    }

    /// Attach an opaque shadow map and atomically install textured GGX visibility shading.
    /// Map production must precede lighting in the same ordered stream.
    /// # Errors
    /// Rejects foreign resources, invalid settings, or shader/pipeline validation.
    pub async fn enable_shadowed_point_light(
        &mut self,
        device: &wgpu::Device,
        map: &crate::ShadowMap,
        settings: crate::ShadowSettings,
    ) -> Result<(), SceneShaderError> {
        self.resource_device(device)
            .map_err(|e| SceneShaderError(e.to_string()))?;
        let candidate = crate::shadow_visibility::ShadowBindings::new(device, map, settings)
            .map_err(|e| SceneShaderError(e.to_string()))?;
        let previous = self.shadow.replace(candidate);
        if let Err(error) = self
            .reload_shader(
                device,
                &crate::environment_lighting::shader_with_environment(self.environment.as_ref()),
            )
            .await
        {
            self.shadow = previous;
            return Err(error);
        }
        Ok(())
    }
    /// Attach initialized GGX environment and DFG resources for single-scattering specular IBL.
    /// Production must precede scene rendering. Diffuse IBL and multiscattering are separate.
    /// # Errors
    /// Rejects foreign resources or shader/pipeline validation; preserves the previous state.
    pub async fn enable_environment_lighting(
        &mut self,
        device: &wgpu::Device,
        environment: &crate::GgxEnvironmentPrefilter,
        dfg: &crate::GgxDfgLut,
    ) -> Result<(), SceneShaderError> {
        let source = crate::environment_lighting::shader(self.shadow.is_some());
        self.enable_environment_source(device, environment, dfg, None, &source)
            .await
    }
    /// Attach specular and normalized diffuse environment cubes with DFG integration.
    /// # Errors
    /// Rejects foreign resources and invalid shaders, preserving the previous configuration.
    pub async fn enable_full_environment_lighting(
        &mut self,
        device: &wgpu::Device,
        environment: &crate::GgxEnvironmentPrefilter,
        dfg: &crate::GgxDfgLut,
        diffuse: &crate::DiffuseEnvironmentConvolution,
    ) -> Result<(), SceneShaderError> {
        let source = crate::environment_lighting::shader_combined(self.shadow.is_some(), true);
        self.enable_environment_source(device, environment, dfg, Some(diffuse), &source)
            .await
    }
    pub(crate) async fn enable_environment_source(
        &mut self,
        device: &wgpu::Device,
        environment: &crate::GgxEnvironmentPrefilter,
        dfg: &crate::GgxDfgLut,
        diffuse: Option<&crate::DiffuseEnvironmentConvolution>,
        source: &str,
    ) -> Result<(), SceneShaderError> {
        self.resource_device(device)
            .map_err(|e| SceneShaderError(e.to_string()))?;
        let intensity = self.environment.as_ref().map_or(1.0, |e| e.intensity);
        let candidate = crate::environment_lighting::EnvironmentBindings::new(
            device,
            environment,
            dfg,
            diffuse,
            intensity,
        )
        .map_err(|e| SceneShaderError(e.to_string()))?;
        let previous = self.environment.replace(candidate);
        if let Err(error) = self.reload_shader(device, source).await {
            self.environment = previous;
            return Err(error);
        }
        Ok(())
    }

    /// Scale diffuse and specular IBL without recomputing environment textures.
    /// Queue must belong to this renderer's device (wgpu validation).
    /// # Errors
    /// Rejects negative/nonfinite intensity or missing environment bindings.
    pub fn set_environment_intensity(
        &mut self,
        queue: &wgpu::Queue,
        intensity: f32,
    ) -> Result<(), SceneShaderError> {
        if !intensity.is_finite() || intensity < 0.0 {
            return Err(SceneShaderError(
                "environment intensity must be finite and nonnegative".into(),
            ));
        }
        let environment = self
            .environment
            .as_mut()
            .ok_or_else(|| SceneShaderError("environment lighting is not attached".into()))?;
        queue.write_buffer(
            &environment.intensity_buffer,
            0,
            bytemuck::cast_slice(&[intensity, 0.0, 0.0, 0.0]),
        );
        environment.intensity = intensity;
        Ok(())
    }

    /// Update shadow settings on the renderer's queue before encoding new consumers.
    /// # Errors
    /// Rejects absent shadow resources or invalid settings before writing GPU memory.
    pub fn update_shadow_settings(
        &self,
        queue: &wgpu::Queue,
        settings: crate::ShadowSettings,
    ) -> Result<(), SceneError> {
        self.shadow
            .as_ref()
            .ok_or(SceneError::InvalidGeometry)?
            .update(queue, settings)
    }

    /// Replaces both pipelines atomically after compilation and ABI validation.
    /// Existing textures and transforms remain compatible. An identical source is
    /// a cache hit and does not compile again. Call from the device owner task.
    ///
    /// # Errors
    /// Returns shader diagnostics for invalid WGSL or a mismatched pipeline ABI;
    /// the previously working shader and revision remain active.
    pub async fn reload_shader(
        &mut self,
        device: &wgpu::Device,
        source: &str,
    ) -> Result<bool, SceneShaderError> {
        if device != &self.device {
            return Err(SceneShaderError("shader reload device mismatch".to_owned()));
        }
        if source == self.shader_source {
            return Ok(false);
        }
        // Allocate on the retained owner even if separate instances reuse device IDs.
        let device = &self.device;
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("custom scene shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        if let Some(error) = scope.pop().await {
            return Err(SceneShaderError(error.to_string()));
        }
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let (world, overlay, transparent) = Self::create_shader_pipelines(
            device,
            self.color_format,
            &self.transform_layout,
            &self.texture_layout,
            &shader,
            1,
            self.shadow.as_ref().map(|shadow| &shadow.layout),
            self.environment
                .as_ref()
                .map(|environment| &environment.layout),
        );
        let msaa = self.msaa_pipelines.as_ref().map(|_| {
            Self::create_shader_pipelines(
                device,
                self.color_format,
                &self.transform_layout,
                &self.texture_layout,
                &shader,
                4,
                self.shadow.as_ref().map(|shadow| &shadow.layout),
                self.environment
                    .as_ref()
                    .map(|environment| &environment.layout),
            )
        });
        if let Some(error) = scope.pop().await {
            return Err(SceneShaderError(error.to_string()));
        }
        self.msaa_pipelines = msaa;
        self.world_pipeline = world;
        self.overlay_pipeline = overlay;
        self.transparent_pipeline = transparent;
        source.clone_into(&mut self.shader_source);
        self.shader_revision = self.shader_revision.saturating_add(1);
        Ok(true)
    }

    #[must_use]
    pub const fn shader_revision(&self) -> u64 {
        self.shader_revision
    }

    #[allow(clippy::too_many_lines)]
    fn create_shader_pipelines(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        transform_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
        shader: &wgpu::ShaderModule,
        sample_count: u32,
        shadow_layout: Option<&wgpu::BindGroupLayout>,
        environment_layout: Option<&wgpu::BindGroupLayout>,
    ) -> (
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
    ) {
        let mut layouts = vec![Some(transform_layout), Some(texture_layout)];
        if shadow_layout.is_some() || environment_layout.is_some() {
            layouts.push(shadow_layout);
        }
        if let Some(environment) = environment_layout {
            layouts.push(Some(environment));
        }
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene pipeline layout"),
            bind_group_layouts: &layouts,
            immediate_size: 0,
        });
        let attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4];
        let normal_attributes = wgpu::vertex_attr_array![3 => Float32x3];
        let coordinate_attributes = wgpu::vertex_attr_array![5 => Float32x3];
        let material_attributes = wgpu::vertex_attr_array![4 => Float32x4];
        let pipeline = |overlay: bool, depth_write: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("scene pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[
                        Some(wgpu::VertexBufferLayout {
                            array_stride: size_of::<SceneVertex>() as u64,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &attributes,
                        }),
                        Some(wgpu::VertexBufferLayout {
                            array_stride: 12,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &normal_attributes,
                        }),
                        Some(wgpu::VertexBufferLayout {
                            array_stride: 16,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &material_attributes,
                        }),
                        Some(wgpu::VertexBufferLayout {
                            array_stride: 12,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &coordinate_attributes,
                        }),
                    ],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(depth_write),
                    depth_compare: Some(if overlay {
                        wgpu::CompareFunction::Always
                    } else {
                        wgpu::CompareFunction::LessEqual
                    }),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: sample_count,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: shader,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        (
            pipeline(false, true),
            pipeline(true, false),
            pipeline(false, false),
        )
    }

    /// Logical buffer bytes required by a validated mesh upload. Includes the
    /// normal/material-coordinate buffers and fixed material parameters.
    #[must_use]
    pub fn mesh_allocation_bytes(mesh: &SceneMesh) -> u64 {
        mesh.vertices().len() as u64 * (std::mem::size_of::<SceneVertex>() as u64 + 24)
            + mesh.indices().len() as u64 * 4
            + 16
    }

    /// # Errors
    /// Rejects a foreign device or invalid geometry, including primitive helper input.
    pub fn upload_shared_mesh_partitions(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
    ) -> Result<Vec<SceneGeometry>, SceneError> {
        self.upload_mesh_index_variants(
            device,
            mesh,
            &[mesh.indices(), mesh.indices()],
            wgpu::BufferUsages::empty(),
        )
    }
    /// Shared draw streams that may be updated by admitted compute transfers.
    pub fn upload_compute_shared_mesh_partitions(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
    ) -> Result<Vec<SceneGeometry>, SceneError> {
        self.upload_mesh_index_variants(
            device,
            mesh,
            &[mesh.indices(), mesh.indices()],
            wgpu::BufferUsages::STORAGE,
        )
    }
    pub async fn set_geometry_opaque_shader(
        &self,
        device: &wgpu::Device,
        geometry: &mut SceneGeometry,
        source: &str,
    ) -> Result<(), SceneShaderError> {
        if device != &self.device || device != &geometry.device {
            return Err(SceneShaderError("geometry material device mismatch".into()));
        }
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("geometry opaque shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let one = Self::create_shader_pipelines(
            device,
            self.color_format,
            &self.transform_layout,
            &self.texture_layout,
            &shader,
            1,
            self.shadow.as_ref().map(|s| &s.layout),
            self.environment.as_ref().map(|e| &e.layout),
        );
        let four = Self::create_shader_pipelines(
            device,
            self.color_format,
            &self.transform_layout,
            &self.texture_layout,
            &shader,
            4,
            self.shadow.as_ref().map(|s| &s.layout),
            self.environment.as_ref().map(|e| &e.layout),
        );
        if let Some(error) = scope.pop().await {
            return Err(SceneShaderError(error.to_string()));
        }
        geometry.opaque_shader = Some((
            self.shader_revision,
            self.transform_layout.clone(),
            one.0,
            four.0,
        ));
        Ok(())
    }
    pub fn upload_mesh(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
    ) -> Result<SceneGeometry, SceneError> {
        self.upload_mesh_with_usage(device, mesh, wgpu::BufferUsages::empty())
    }

    /// Fixed topology with vertex and normal storage writable by admitted compute work.
    pub fn upload_compute_mesh(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
    ) -> Result<SceneGeometry, SceneError> {
        self.upload_mesh_with_usage(device, mesh, wgpu::BufferUsages::STORAGE)
    }

    /// Keep a fixed mesh and sparse displacement binding resident on the GPU.
    /// Only control-node displacements are uploaded per pose; rendering consumes
    /// the resulting geometry without CPU vertex readback.
    pub fn prepare_surface_deformation(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
        offsets: &[u32],
        weights: &[crate::SurfaceDeformationWeight],
        control_count: u32,
    ) -> Result<crate::SurfaceDeformation, SceneError> {
        crate::surface_deformation::validate(device, mesh, offsets, weights, control_count)?;
        let geometry = self.upload_mesh_with_usage(
            device,
            mesh,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        )?;
        crate::SurfaceDeformation::new(device, mesh, geometry, offsets, weights, control_count)
    }

    fn upload_mesh_with_usage(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
        additional_vertex_usage: wgpu::BufferUsages,
    ) -> Result<SceneGeometry, SceneError> {
        let mut geometry = self.upload_mesh_index_variants(
            device,
            mesh,
            &[mesh.indices()],
            additional_vertex_usage,
        )?;
        Ok(geometry.remove(0))
    }

    fn upload_mesh_index_variants(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
        variants: &[&[u32]],
        additional_vertex_usage: wgpu::BufferUsages,
    ) -> Result<Vec<SceneGeometry>, SceneError> {
        Ok(self
            .upload_mesh_buffer_batch(device, mesh, variants, additional_vertex_usage, &[])?
            .geometries)
    }

    fn upload_mesh_buffer_batch(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
        variants: &[&[u32]],
        additional_vertex_usage: wgpu::BufferUsages,
        additional_buffers: &[voxy_gpu_core::ManagedBufferDescriptor<'_>],
    ) -> Result<UploadedGeometryBatch, SceneError> {
        use voxy_gpu_core::ManagedBufferDescriptor;
        let device = self.resource_device(device)?;
        mesh.validate_for_upload()?;
        if variants.is_empty() {
            return Err(SceneError::InvalidGeometry);
        }
        for indices in variants {
            geometry_sizes(device, mesh.vertices.len(), indices.len())?;
            if indices.iter().any(|i| *i as usize >= mesh.vertices.len()) {
                return Err(SceneError::InvalidIndex);
            }
        }
        let normal_cache = NormalCache::from_mesh(mesh);
        let coordinates = mesh.material_coordinates();
        let vertex_usage = wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST;
        let data = [
            bytemuck::cast_slice(&mesh.vertices),
            bytemuck::cast_slice(&normal_cache.normals),
            bytemuck::cast_slice(coordinates.as_ref()),
            bytemuck::cast_slice(&mesh.material_parameters),
        ];
        let labels = [
            "scene vertices",
            "scene smooth normals",
            "scene material coordinates",
            "scene material parameters",
        ];
        let usages = [
            vertex_usage | additional_vertex_usage,
            vertex_usage | additional_vertex_usage,
            vertex_usage,
            vertex_usage,
        ];
        let mut descriptors: Vec<_> = (0..4)
            .map(|i| ManagedBufferDescriptor {
                label: labels[i],
                size: data[i].len() as u64,
                contents: Some(data[i]),
                usage: usages[i],
            })
            .collect();
        descriptors.extend(variants.iter().map(|indices| ManagedBufferDescriptor {
            label: "scene indices",
            size: indices.len() as u64 * 4,
            contents: Some(bytemuck::cast_slice(indices)),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        }));
        descriptors.extend_from_slice(additional_buffers);
        let mut buffers = crate::ComputeMemoryBudget::for_device(device)
            .allocate_buffer_batch(&descriptors)
            .map_err(scene_memory_error)?
            .into_iter()
            .map(std::sync::Arc::new);
        let vertices = buffers.next().expect("validated vertex stream");
        let normals = buffers.next().expect("validated normal stream");
        let material_coordinates = buffers.next().expect("validated coordinate stream");
        let material_parameters = buffers.next().expect("validated parameter stream");
        let mut cache = Some((normal_cache, coordinates.into_owned()));
        let geometries = buffers
            .by_ref()
            .take(variants.len())
            .zip(variants)
            .map(|(indices, source)| {
                let (normal_cache, coordinate_cache) = cache.take().unwrap_or_default();
                SceneGeometry {
                    device: device.clone(),
                    vertices: vertices.clone(),
                    normals: normals.clone(),
                    normal_cache,
                    material_coordinates: material_coordinates.clone(),
                    coordinate_cache,
                    material_parameters: material_parameters.clone(),
                    indices,
                    index_count: source.len() as u32,
                    depth_mode: SceneDepthMode::Opaque,
                    opaque_shader: None,
                    partitioned_indices: false,
                    partition_cache: Vec::new(),
                    vertex_capacity: mesh.vertices.len(),
                    index_capacity: source.len(),
                    identity: next_geometry_identity(),
                    revision: 0,
                }
            })
            .collect();
        Ok(UploadedGeometryBatch {
            geometries,
            additional_buffers: buffers.collect(),
        })
    }

    /// Replace published geometry only after validating and uploading its replacement.
    /// Invalid imports and foreign devices leave the existing GPU geometry intact.
    /// The target's depth mode is preserved across successful replacement.
    /// # Errors
    /// Preserves upload validation errors before changing the target geometry.
    pub fn replace_mesh(
        &self,
        device: &wgpu::Device,
        target: &mut SceneGeometry,
        mesh: &SceneMesh,
    ) -> Result<(), SceneError> {
        let mut replacement = self.upload_mesh(device, mesh)?;
        replacement.depth_mode = target.depth_mode;
        *target = replacement;
        Ok(())
    }

    /// Reserves empty GPU geometry for dynamic meshes/sprite batches.
    /// Call `SceneGeometry::update` before drawing; `clear` reuses the buffers.
    /// # Errors
    /// Rejects a foreign device, zero, overflow or excessive buffer capacities.
    pub fn reserve_geometry(
        &self,
        device: &wgpu::Device,
        vertex_capacity: usize,
        index_capacity: usize,
    ) -> Result<SceneGeometry, SceneError> {
        let device = self.resource_device(device)?;
        let (vertex_bytes, index_bytes) = geometry_sizes(device, vertex_capacity, index_capacity)?;
        use voxy_gpu_core::ManagedBufferDescriptor;
        let vertex_usage = wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST;
        let parameters = [1.333_f32, 0.2, 0.08, 0.04];
        let sizes = [
            vertex_bytes,
            vertex_capacity as u64 * 12,
            vertex_capacity as u64 * 12,
            16,
            index_bytes,
        ];
        let labels = [
            "reserved scene vertices",
            "reserved scene normals",
            "reserved material coordinates",
            "reserved scene material parameters",
            "reserved scene indices",
        ];
        let descriptors = std::array::from_fn(|i| ManagedBufferDescriptor {
            label: labels[i],
            size: sizes[i],
            contents: (i == 3).then(|| bytemuck::cast_slice(&parameters)),
            usage: if i == 4 {
                wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST
            } else {
                vertex_usage
            },
        });
        let [
            vertices,
            normals,
            material_coordinates,
            material_parameters,
            indices,
        ] = crate::ComputeMemoryBudget::for_device(device)
            .allocate_buffers(descriptors)
            .map_err(scene_memory_error)?
            .map(std::sync::Arc::new);
        Ok(SceneGeometry {
            device: device.clone(),
            vertices,
            normals,
            material_coordinates,
            material_parameters,
            indices,
            normal_cache: NormalCache::default(),
            coordinate_cache: Vec::new(),
            index_count: 0,
            depth_mode: SceneDepthMode::Opaque,
            opaque_shader: None,
            partitioned_indices: false,
            partition_cache: Vec::new(),
            vertex_capacity,
            index_capacity,
            identity: next_geometry_identity(),
            revision: 0,
        })
    }

    /// Upload a decoded PNG/JPEG asset with explicit material sampling.
    /// The queue must belong to this renderer's device.
    /// # Errors
    /// Rejects a foreign device or dimensions exceeding GPU limits.
    pub fn upload_image(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &crate::ImageAsset,
        sampling: TextureSampling,
    ) -> Result<SceneTexture, SceneError> {
        self.upload_texture_with_sampling(
            device,
            queue,
            image.width(),
            image.height(),
            image.rgba(),
            sampling,
        )
    }

    /// Uploads RGBA8 sRGB texels. A 1x1 white texture gives vertex-color materials.
    /// # Errors
    /// Rejects zero size, unsupported dimensions and wrong byte counts.
    pub fn upload_texture(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<SceneTexture, SceneError> {
        self.upload_texture_with_sampling(
            device,
            queue,
            width,
            height,
            rgba,
            TextureSampling::default(),
        )
    }

    /// Uploads a single-level sRGB texture with explicit addressing/filtering.
    /// The queue must belong to the renderer's device.
    /// # Errors
    /// Rejects a foreign device, dimensions or byte counts before allocation/writes.
    #[allow(clippy::too_many_arguments)]
    pub fn upload_texture_with_sampling(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        rgba: &[u8],
        sampling: TextureSampling,
    ) -> Result<SceneTexture, SceneError> {
        self.upload_texture_levels(device, queue, &[(width, height, rgba)], sampling)
    }

    /// Upload a complete authored mip chain, ordered from base image to 1x1.
    /// RGB is interpreted as sRGB and alpha remains linear. The supplied queue
    /// must belong to this renderer's device.
    /// # Errors
    /// Rejects empty/incomplete chains, wrong level dimensions or foreign devices
    /// before allocating or writing GPU resources.
    pub fn upload_image_mips(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        images: &[crate::ImageAsset],
        sampling: TextureSampling,
    ) -> Result<SceneTexture, SceneError> {
        let levels: Vec<_> = images
            .iter()
            .map(|image| (image.width(), image.height(), image.rgba()))
            .collect();
        let Some(&(width, height, _)) = levels.first() else {
            return Err(SceneError::InvalidTexture);
        };
        if levels.len()
            != usize::try_from(width.max(height).ilog2() + 1)
                .map_err(|_| SceneError::InvalidTexture)?
        {
            return Err(SceneError::InvalidTexture);
        }
        self.upload_texture_levels(device, queue, &levels, sampling)
    }

    fn upload_texture_levels(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        levels: &[(u32, u32, &[u8])],
        sampling: TextureSampling,
    ) -> Result<SceneTexture, SceneError> {
        if !(1..=16).contains(&sampling.anisotropy)
            || (sampling.anisotropy > 1
                && (sampling.min_filter != TextureFilter::Linear
                    || sampling.mag_filter != TextureFilter::Linear
                    || sampling.mipmap_filter.unwrap_or(sampling.min_filter)
                        != TextureFilter::Linear))
        {
            return Err(SceneError::InvalidTexture);
        }
        let &(width, height, _) = levels.first().ok_or(SceneError::InvalidTexture)?;
        let device = self.resource_device(device)?;
        for (level, &(w, h, rgba)) in levels.iter().enumerate() {
            let shift = u32::try_from(level).map_err(|_| SceneError::InvalidTexture)?;
            let expected = u64::from(w)
                .checked_mul(u64::from(h))
                .and_then(|n| n.checked_mul(4));
            if w == 0
                || h == 0
                || w != (width >> shift).max(1)
                || h != (height >> shift).max(1)
                || w > device.limits().max_texture_dimension_2d
                || h > device.limits().max_texture_dimension_2d
                || expected.and_then(|n| usize::try_from(n).ok()) != Some(rgba.len())
            {
                return Err(SceneError::InvalidTexture);
            }
        }
        let texture = crate::ComputeMemoryBudget::for_device(device)
            .allocate_texture(&wgpu::TextureDescriptor {
                label: Some("scene image"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: u32::try_from(levels.len())
                    .map_err(|_| SceneError::InvalidTexture)?,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            })
            .map_err(|error| match error {
                crate::ComputeError::MemoryBudget => SceneError::MemoryBudget,
                _ => SceneError::InvalidTexture,
            })?;
        for (level, &(w, h, rgba)) in levels.iter().enumerate() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: u32::try_from(level).map_err(|_| SceneError::InvalidTexture)?,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(w * 4),
                    rows_per_image: Some(h),
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
        }
        self.bind_image(
            device,
            texture,
            sampling,
            u32::try_from(levels.len()).map_err(|_| SceneError::InvalidTexture)?,
        )
    }

    /// Create another material binding without uploading or copying image pixels.
    /// `mip_levels` restricts the view, allowing base-only and mip-filtered materials
    /// to share the same allocation.
    ///
    /// # Errors
    /// Rejects foreign devices, invalid sampling or an unavailable mip range.
    /// Create a filterable colour attachment that can be sampled in a later pass.
    /// # Errors
    /// Rejects foreign devices, unsupported formats and invalid dimensions.
    pub fn create_sampled_color(
        &self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> Result<SceneTexture, SceneError> {
        let device = self.resource_device(device)?;
        if width == 0
            || height == 0
            || width > device.limits().max_texture_dimension_2d
            || height > device.limits().max_texture_dimension_2d
            || !matches!(
                self.color_format,
                wgpu::TextureFormat::Rgba8Unorm
                    | wgpu::TextureFormat::Rgba8UnormSrgb
                    | wgpu::TextureFormat::Bgra8Unorm
                    | wgpu::TextureFormat::Bgra8UnormSrgb
                    | wgpu::TextureFormat::Rgba16Float
            )
        {
            return Err(SceneError::InvalidTexture);
        }
        let texture = crate::ComputeMemoryBudget::for_device(device)
            .allocate_texture(&wgpu::TextureDescriptor {
                label: Some("sampled opaque scene colour"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.color_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            })
            .map_err(|error| match error {
                crate::ComputeError::MemoryBudget => SceneError::MemoryBudget,
                _ => SceneError::InvalidTexture,
            })?;
        self.bind_image(
            device,
            texture,
            TextureSampling {
                min_filter: TextureFilter::Linear,
                mag_filter: TextureFilter::Linear,
                ..TextureSampling::default()
            },
            1,
        )
    }

    /// Create sampler bindings sharing an existing uploaded image.
    /// # Errors
    /// Rejects foreign devices/images, invalid mip counts and unsupported sampling.
    pub fn texture_binding(
        &self,
        device: &wgpu::Device,
        image: &SceneTexture,
        sampling: TextureSampling,
        mip_levels: u32,
    ) -> Result<SceneTexture, SceneError> {
        let device = self.resource_device(device)?;
        if !image.belongs_to(device) {
            return Err(SceneError::DeviceMismatch);
        }
        self.bind_image(device, image.texture.clone(), sampling, mip_levels)
    }

    pub(crate) fn bind_image(
        &self,
        device: &wgpu::Device,
        texture: std::sync::Arc<voxy_gpu_core::ManagedTexture>,
        sampling: TextureSampling,
        mip_levels: u32,
    ) -> Result<SceneTexture, SceneError> {
        let device = self.resource_device(device)?;
        if mip_levels == 0
            || mip_levels > texture.mip_level_count()
            || !(1..=16).contains(&sampling.anisotropy)
            || (sampling.anisotropy > 1
                && (sampling.min_filter != TextureFilter::Linear
                    || sampling.mag_filter != TextureFilter::Linear
                    || sampling.mipmap_filter.unwrap_or(sampling.min_filter)
                        != TextureFilter::Linear))
        {
            return Err(SceneError::InvalidTexture);
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            mip_level_count: Some(mip_levels),
            ..wgpu::TextureViewDescriptor::default()
        });
        let sampler = device.create_sampler(&sampling.descriptor());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene material"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        Ok(SceneTexture {
            device: device.clone(),
            bind_group,
            texture,
            sampler,
        })
    }

    /// # Errors
    /// Rejects a foreign device, non-finite matrices or shared memory exhaustion.
    pub fn create_transform(
        &self,
        device: &wgpu::Device,
        mvp: Mat4,
    ) -> Result<SceneTransform, SceneError> {
        let device = self.resource_device(device)?;
        if !mvp.is_finite() {
            return Err(SceneError::InvalidTransform);
        }
        let mut uniform = Vec::with_capacity(64);
        uniform.extend(mvp.to_cols_array());
        uniform.extend(mvp.to_cols_array());
        uniform.extend([0., 0., 2., 1.]);
        uniform.extend(Mat4::IDENTITY.to_cols_array());
        uniform.extend([1.; 4]);
        uniform.extend([0., 0., 1., 0.]);
        uniform.extend([0.; 4]);
        let contents = bytemuck::cast_slice(&uniform);
        let [buffer] = crate::ComputeMemoryBudget::for_device(device)
            .allocate_buffers([voxy_gpu_core::ManagedBufferDescriptor {
                label: "scene MVP and material view",
                size: contents.len() as u64,
                contents: Some(contents),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }])
            .map_err(scene_memory_error)?;
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene transform"),
            layout: &self.transform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        Ok(SceneTransform { buffer, bind_group })
    }

    /// Reveals internals with their own depth buffer, preserving self-occlusion.
    /// `xray_depth` must be a distinct `Depth32Float` target matching the color size.
    /// Opaque X-ray surfaces are order independent. Transparent internals still
    /// require sorting; UI is composed last. World depth remains untouched.
    #[allow(clippy::too_many_arguments)]
    pub fn encode_with_xray_depth(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        xray_depth: &wgpu::TextureView,
        clear: wgpu::Color,
        draws: &[SceneDraw<'_>],
    ) {
        let world: Vec<_> = draws
            .iter()
            .filter(|d| !d.overlay && d.geometry.depth_mode != SceneDepthMode::Xray)
            .map(|d| SceneDraw {
                geometry: d.geometry,
                texture: d.texture,
                transform: d.transform,
                overlay: false,
            })
            .collect();
        self.encode(encoder, color, depth, clear, &world);
        for overlay in [false, true] {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(if overlay {
                    "X-ray UI"
                } else {
                    "isolated X-ray internals"
                }),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: if overlay { depth } else { xray_depth },
                    depth_ops: Some(wgpu::Operations {
                        load: if overlay {
                            wgpu::LoadOp::Load
                        } else {
                            wgpu::LoadOp::Clear(1.0)
                        },
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            self.encode_draw_stages(
                &mut pass,
                draws,
                (
                    &self.world_pipeline,
                    &self.overlay_pipeline,
                    &self.transparent_pipeline,
                ),
                if overlay { 3..4 } else { 2..3 },
                !overlay,
            );
        }
    }

    /// Draw UI/overlay layers over existing composed color without clearing it.
    /// Depth is retained; overlay pipeline ignores depth and does not write it.
    pub fn encode_overlays(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        draws: &[SceneDraw<'_>],
    ) {
        self.encode_overlays_region(encoder, target, depth, draws, None);
    }

    /// Draw per-view overlays after composition into a full-sized depth/color target.
    pub fn encode_overlays_viewport(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        draws: &[SceneDraw<'_>],
        viewport: [u32; 4],
    ) {
        self.encode_overlays_region(encoder, target, depth, draws, Some(viewport));
    }

    fn encode_overlays_region(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        draws: &[SceneDraw<'_>],
        viewport: Option<[u32; 4]>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene overlays after composition"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        if let Some([x, y, w, h]) = viewport {
            pass.set_viewport(x as f32, y as f32, w as f32, h as f32, 0., 1.);
            pass.set_scissor_rect(x, y, w, h);
        }
        pass.set_pipeline(&self.overlay_pipeline);
        for draw in draws.iter().filter(|draw| draw.overlay) {
            if let Some(shadow) = &self.shadow {
                pass.set_bind_group(2, &shadow.group, &[]);
            }
            if let Some(environment) = &self.environment {
                pass.set_bind_group(3, &environment.group, &[]);
            }
            pass.set_bind_group(0, &draw.transform.bind_group, &[]);
            pass.set_bind_group(1, &draw.texture.bind_group, &[]);
            pass.set_vertex_buffer(0, draw.geometry.vertices.slice(..));
            pass.set_vertex_buffer(1, draw.geometry.normals.slice(..));
            pass.set_vertex_buffer(2, draw.geometry.material_parameters.slice(..));
            pass.set_vertex_buffer(3, draw.geometry.material_coordinates.slice(..));
            pass.set_index_buffer(draw.geometry.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..draw.geometry.index_count, 0, 0..1);
        }
    }

    /// Draw depth-tested layers over existing colour/depth without clearing or depth writes.
    pub fn encode_transparent_over(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        draws: &[SceneDraw<'_>],
    ) {
        self.encode_transparent_pass(
            encoder,
            target,
            depth,
            None,
            draws,
            &self.transparent_pipeline,
        );
    }
    /// Composite depth-tested four-sample layers and resolve to a single-sample target.
    /// # Errors
    /// Rejects renderers without four-sample pipelines before recording commands.
    pub fn encode_transparent_over_msaa4(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        resolve: &wgpu::TextureView,
        draws: &[SceneDraw<'_>],
    ) -> Result<(), SceneError> {
        let pipelines = self
            .msaa_pipelines
            .as_ref()
            .ok_or(SceneError::MultisamplingNotEnabled)?;
        self.encode_transparent_pass(encoder, target, depth, Some(resolve), draws, &pipelines.2);
        Ok(())
    }
    fn encode_transparent_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        resolve: Option<&wgpu::TextureView>,
        draws: &[SceneDraw<'_>],
        pipeline: &wgpu::RenderPipeline,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("transparent scene colour composition"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: resolve,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        for draw in draws.iter().filter(|draw| !draw.overlay) {
            if let Some(shadow) = &self.shadow {
                pass.set_bind_group(2, &shadow.group, &[]);
            }
            if let Some(environment) = &self.environment {
                pass.set_bind_group(3, &environment.group, &[]);
            }
            pass.set_bind_group(0, &draw.transform.bind_group, &[]);
            pass.set_bind_group(1, &draw.texture.bind_group, &[]);
            pass.set_vertex_buffer(0, draw.geometry.vertices.slice(..));
            pass.set_vertex_buffer(1, draw.geometry.normals.slice(..));
            pass.set_vertex_buffer(2, draw.geometry.material_parameters.slice(..));
            pass.set_vertex_buffer(3, draw.geometry.material_coordinates.slice(..));
            pass.set_index_buffer(draw.geometry.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..draw.geometry.index_count, 0, 0..1);
        }
    }

    /// Records opaque, transparent, X-ray and overlay layers in that order.
    /// Preserves caller order within each layer; alpha geometry needs back-to-front sorting.
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        clear: wgpu::Color,
        draws: &[SceneDraw<'_>],
    ) {
        self.encode_pass(
            encoder,
            color,
            depth,
            None,
            clear,
            draws,
            (
                &self.world_pipeline,
                &self.overlay_pipeline,
                &self.transparent_pipeline,
            ),
            None,
        );
    }

    /// Timestamp the ordinary single-sample scene pass. Query indices/type and
    /// device must satisfy wgpu validation. Does not measure presentation or
    /// other shadow/composition passes. Rejects unavailable timing before encoding.
    #[allow(clippy::too_many_arguments)]
    pub fn encode_profiled(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        clear: wgpu::Color,
        draws: &[SceneDraw<'_>],
        timestamps: wgpu::RenderPassTimestampWrites<'_>,
    ) -> Result<(), SceneError> {
        if !self
            .device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY)
        {
            return Err(SceneError::TimestampQueriesNotEnabled);
        }
        self.encode_pass(
            encoder,
            color,
            depth,
            None,
            clear,
            draws,
            (
                &self.world_pipeline,
                &self.overlay_pipeline,
                &self.transparent_pipeline,
            ),
            Some(timestamps),
        );
        Ok(())
    }

    /// Records disjoint views in one pass, clearing color/depth once. Each view
    /// has its own viewport and scissor; its overlays cannot escape that region.
    /// Views must reference base-mip, single-sample color/depth targets of equal
    /// dimensions. Separate transforms are required when cameras differ.
    /// # Errors
    /// Rejects empty, overflowing, out-of-target or overlapping regions before
    /// recording commands. Overlap requires separate depth targets/composition.
    pub fn encode_views(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        clear: wgpu::Color,
        views: &[SceneView<'_>],
    ) -> Result<(), SceneError> {
        self.encode_view_pass(
            encoder,
            color,
            depth,
            views,
            ViewPass {
                resolve: None,
                color_load: wgpu::LoadOp::Clear(clear),
                depth_load: wgpu::LoadOp::Clear(1.),
                pipelines: (
                    &self.world_pipeline,
                    &self.overlay_pipeline,
                    &self.transparent_pipeline,
                ),
                stages: 0..4,
                isolated_xray: false,
            },
        )
    }

    /// Resolves all disjoint views together after one four-sample pass.
    /// # Errors
    /// Rejects missing MSAA pipelines, inconsistent sample counts/target sizes,
    /// or invalid view regions before recording commands.
    #[allow(clippy::too_many_arguments)]
    pub fn encode_views_msaa4(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        resolve: &wgpu::TextureView,
        clear: wgpu::Color,
        views: &[SceneView<'_>],
    ) -> Result<(), SceneError> {
        let (world, overlay, transparent) = self
            .msaa_pipelines
            .as_ref()
            .ok_or(SceneError::MultisamplingNotEnabled)?;
        self.encode_view_pass(
            encoder,
            color,
            depth,
            views,
            ViewPass {
                resolve: Some(resolve),
                color_load: wgpu::LoadOp::Clear(clear),
                depth_load: wgpu::LoadOp::Clear(1.),
                pipelines: (world, overlay, transparent),
                stages: 0..4,
                isolated_xray: false,
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn encode_view_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        views: &[SceneView<'_>],
        setup: ViewPass<'_>,
    ) -> Result<(), SceneError> {
        let size = color.texture().size();
        if size != depth.texture().size()
            || color.texture().sample_count() != if setup.resolve.is_some() { 4 } else { 1 }
            || depth.texture().sample_count() != color.texture().sample_count()
        {
            return Err(SceneError::InvalidGeometry);
        }
        if setup.resolve.is_some_and(|target| {
            target.texture().size() != size || target.texture().sample_count() != 1
        }) {
            return Err(SceneError::InvalidGeometry);
        }
        validate_view_regions([size.width, size.height], views)?;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("disjoint scene views"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color,
                depth_slice: None,
                resolve_target: setup.resolve,
                ops: wgpu::Operations {
                    load: setup.color_load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: setup.depth_load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        for view in views {
            let [x, y, width, height] = view.viewport;
            pass.set_viewport(x as f32, y as f32, width as f32, height as f32, 0., 1.);
            pass.set_scissor_rect(x, y, width, height);
            self.encode_draw_stages(
                &mut pass,
                view.draws,
                setup.pipelines,
                setup.stages.clone(),
                setup.isolated_xray,
            );
        }
        Ok(())
    }

    /// Composes viewport-local content followed by whole-target editor/game UI.
    /// The final overlay pass starts with the target's full viewport, so it does
    /// not inherit the last scene view's scissor or viewport.
    /// # Errors
    /// Rejects non-overlay global draws and invalid view regions before encoding.
    #[allow(clippy::too_many_arguments)]
    pub fn encode_view_frame(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        clear: wgpu::Color,
        views: &[SceneView<'_>],
        overlays: &[SceneDraw<'_>],
    ) -> Result<(), SceneError> {
        self.encode_view_frame_targets(
            encoder,
            SceneViewTargets {
                color,
                depth,
                xray_depth: None,
                resolve: None,
                overlay_depth: depth,
            },
            clear,
            views,
            overlays,
        )
    }

    /// Four-sample scene views followed by single-sample full-window UI. UI uses
    /// its own depth attachment and does not read or overwrite world depth.
    /// # Errors
    /// Rejects non-overlay global draws, invalid targets/regions and missing MSAA.
    #[allow(clippy::too_many_arguments)]
    pub fn encode_view_frame_msaa4(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        msaa: (&wgpu::TextureView, &wgpu::TextureView),
        output: &wgpu::TextureView,
        overlay_depth: &wgpu::TextureView,
        clear: wgpu::Color,
        views: &[SceneView<'_>],
        overlays: &[SceneDraw<'_>],
    ) -> Result<(), SceneError> {
        self.encode_view_frame_targets(
            encoder,
            SceneViewTargets {
                color: msaa.0,
                depth: msaa.1,
                xray_depth: None,
                resolve: Some(output),
                overlay_depth,
            },
            clear,
            views,
            overlays,
        )
    }

    /// Composes disjoint worlds, self-occluding X-ray internals, view-local UI,
    /// then global UI. World depth is never cleared by the X-ray/UI passes.
    /// # Errors
    /// Validates regions, sizes, sample counts and mandatory nonaliased X-ray
    /// depth before encoding. Views use matching base-mip attachment formats.
    pub fn encode_view_frame_targets(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        targets: SceneViewTargets<'_>,
        clear: wgpu::Color,
        views: &[SceneView<'_>],
        overlays: &[SceneDraw<'_>],
    ) -> Result<(), SceneError> {
        let size = targets.color.texture().size();
        let samples = if targets.resolve.is_some() { 4 } else { 1 };
        let output = targets.resolve.unwrap_or(targets.color);
        let has_xray = views
            .iter()
            .flat_map(|view| view.draws)
            .any(|draw| !draw.overlay && draw.geometry.depth_mode == SceneDepthMode::Xray);
        if overlays.iter().any(|draw| !draw.overlay)
            || targets.color.texture().sample_count() != samples
            || targets.depth.texture().size() != size
            || targets.depth.texture().sample_count() != samples
            || targets.depth.texture().format() != wgpu::TextureFormat::Depth32Float
            || output.texture().size() != size
            || output.texture().sample_count() != 1
            || targets.overlay_depth.texture().size() != size
            || targets.overlay_depth.texture().sample_count() != 1
            || targets.overlay_depth.texture().format() != wgpu::TextureFormat::Depth32Float
            || (has_xray && targets.xray_depth.is_none())
            || targets.xray_depth.is_some_and(|depth| {
                depth.texture() == targets.depth.texture()
                    || depth.texture().size() != size
                    || depth.texture().sample_count() != samples
                    || depth.texture().format() != wgpu::TextureFormat::Depth32Float
            })
        {
            return Err(SceneError::InvalidGeometry);
        }
        validate_view_regions([size.width, size.height], views)?;
        let pipelines = if samples == 4 {
            let (world, overlay, transparent) = self
                .msaa_pipelines
                .as_ref()
                .ok_or(SceneError::MultisamplingNotEnabled)?;
            (world, overlay, transparent)
        } else {
            (
                &self.world_pipeline,
                &self.overlay_pipeline,
                &self.transparent_pipeline,
            )
        };
        self.encode_view_pass(
            encoder,
            targets.color,
            targets.depth,
            views,
            ViewPass {
                resolve: targets.resolve,
                color_load: wgpu::LoadOp::Clear(clear),
                depth_load: wgpu::LoadOp::Clear(1.),
                pipelines,
                stages: if has_xray { 0..2 } else { 0..4 },
                isolated_xray: false,
            },
        )?;
        if has_xray {
            self.encode_view_pass(
                encoder,
                targets.color,
                targets.xray_depth.unwrap(),
                views,
                ViewPass {
                    resolve: targets.resolve,
                    color_load: wgpu::LoadOp::Load,
                    depth_load: wgpu::LoadOp::Clear(1.),
                    pipelines,
                    stages: 2..3,
                    isolated_xray: true,
                },
            )?;
            if views
                .iter()
                .flat_map(|view| view.draws)
                .any(|draw| draw.overlay)
            {
                self.encode_view_pass(
                    encoder,
                    targets.color,
                    targets.depth,
                    views,
                    ViewPass {
                        resolve: targets.resolve,
                        color_load: wgpu::LoadOp::Load,
                        depth_load: wgpu::LoadOp::Load,
                        pipelines,
                        stages: 3..4,
                        isolated_xray: false,
                    },
                )?;
            }
        }
        if !overlays.is_empty() {
            self.encode_overlays(encoder, output, targets.overlay_depth, overlays);
        }
        Ok(())
    }

    /// Compose refractive layers against this frame's shaded world, then draw UI.
    /// The sampled background must be distinct from the output. Layer shaders
    /// receive it as their texture. Re-rendering the world into the output avoids
    /// requiring `COPY_DST` usage on acquired surface textures.
    /// # Errors
    /// Rejects a foreign background, unsupported MSAA, X-ray world geometry,
    /// or overlay layers before recording commands.
    #[allow(clippy::too_many_arguments)]
    pub fn encode_refractive_layers(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        msaa: Option<(&wgpu::TextureView, &wgpu::TextureView)>,
        background: &SceneTexture,
        clear: wgpu::Color,
        draws: &[SceneDraw<'_>],
        layers: &[SceneDraw<'_>],
    ) -> Result<(), SceneError> {
        if !background.belongs_to(&self.device) {
            return Err(SceneError::DeviceMismatch);
        }
        if msaa.is_some() && self.msaa_pipelines.is_none() {
            return Err(SceneError::MultisamplingNotEnabled);
        }
        if draws
            .iter()
            .any(|d| !d.overlay && d.geometry.depth_mode() == SceneDepthMode::Xray)
            || layers.iter().any(|d| d.overlay)
        {
            return Err(SceneError::InvalidGeometry);
        }
        let world: Vec<_> = draws
            .iter()
            .filter(|d| !d.overlay)
            .map(|d| SceneDraw {
                geometry: d.geometry,
                texture: d.texture,
                transform: d.transform,
                overlay: false,
            })
            .collect();
        let wet: Vec<_> = layers
            .iter()
            .map(|d| SceneDraw {
                geometry: d.geometry,
                texture: background,
                transform: d.transform,
                overlay: false,
            })
            .collect();
        let sampled = background
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        if let Some((color, multisampled_depth)) = msaa {
            self.encode_msaa4(encoder, color, multisampled_depth, &sampled, clear, &world)?;
            self.encode_msaa4(encoder, color, multisampled_depth, output, clear, &world)?;
            self.encode_transparent_over_msaa4(encoder, color, multisampled_depth, output, &wet)?;
        } else {
            self.encode(encoder, &sampled, depth, clear, &world);
            self.encode(encoder, output, depth, clear, &world);
            self.encode_transparent_over(encoder, output, depth, &wet);
        }
        self.encode_overlays(encoder, output, depth, draws);
        Ok(())
    }

    /// Resolves four-sample geometry coverage into a single-sample color target.
    /// # Errors
    /// Rejects renderers created without four-sample pipelines.
    #[allow(clippy::too_many_arguments)]
    pub fn encode_msaa4(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        resolve: &wgpu::TextureView,
        clear: wgpu::Color,
        draws: &[SceneDraw<'_>],
    ) -> Result<(), SceneError> {
        let (world, overlay, transparent) = self
            .msaa_pipelines
            .as_ref()
            .ok_or(SceneError::MultisamplingNotEnabled)?;
        self.encode_pass(
            encoder,
            color,
            depth,
            Some(resolve),
            clear,
            draws,
            (world, overlay, transparent),
            None,
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn encode_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        resolve: Option<&wgpu::TextureView>,
        clear: wgpu::Color,
        draws: &[SceneDraw<'_>],
        pipelines: (
            &wgpu::RenderPipeline,
            &wgpu::RenderPipeline,
            &wgpu::RenderPipeline,
        ),
        timestamp_writes: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("general 2D/3D scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color,
                depth_slice: None,
                resolve_target: resolve,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes,
            ..Default::default()
        });
        self.encode_draws(&mut pass, draws, pipelines);
    }

    fn encode_draws(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        draws: &[SceneDraw<'_>],
        pipelines: (
            &wgpu::RenderPipeline,
            &wgpu::RenderPipeline,
            &wgpu::RenderPipeline,
        ),
    ) {
        self.encode_draw_stages(pass, draws, pipelines, 0..4, false);
    }

    fn encode_draw_stages(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        draws: &[SceneDraw<'_>],
        pipelines: (
            &wgpu::RenderPipeline,
            &wgpu::RenderPipeline,
            &wgpu::RenderPipeline,
        ),
        stages: std::ops::Range<u8>,
        isolated_xray: bool,
    ) {
        // Opaque depth first, then ghost shells, revealed internals, finally UI.
        for stage in stages {
            pass.set_pipeline(match stage {
                0 => pipelines.0,
                1 => pipelines.2,
                2 if isolated_xray => pipelines.0,
                _ => pipelines.1,
            });
            for draw in draws.iter().filter(|draw| {
                let draw_stage = if draw.overlay {
                    3
                } else {
                    match draw.geometry.depth_mode {
                        SceneDepthMode::Opaque => 0,
                        SceneDepthMode::Transparent => 1,
                        SceneDepthMode::Xray => 2,
                    }
                };
                draw_stage == stage
            }) {
                if stage == 0 {
                    let msaa = self
                        .msaa_pipelines
                        .as_ref()
                        .is_some_and(|p| std::ptr::eq(pipelines.0, &raw const p.0));
                    pass.set_pipeline(
                        draw.geometry
                            .opaque_shader
                            .as_ref()
                            .filter(|p| p.0 == self.shader_revision && p.1 == self.transform_layout)
                            .map_or(pipelines.0, |p| if msaa { &p.3 } else { &p.2 }),
                    );
                }
                if let Some(shadow) = &self.shadow {
                    pass.set_bind_group(2, &shadow.group, &[]);
                }
                if let Some(environment) = &self.environment {
                    pass.set_bind_group(3, &environment.group, &[]);
                }
                pass.set_bind_group(0, &draw.transform.bind_group, &[]);
                pass.set_bind_group(1, &draw.texture.bind_group, &[]);
                pass.set_vertex_buffer(0, draw.geometry.vertices.slice(..));
                pass.set_vertex_buffer(1, draw.geometry.normals.slice(..));
                pass.set_vertex_buffer(2, draw.geometry.material_parameters.slice(..));
                pass.set_vertex_buffer(3, draw.geometry.material_coordinates.slice(..));
                pass.set_index_buffer(draw.geometry.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..draw.geometry.index_count, 0, 0..1);
            }
        }
    }
}

fn validate_view_regions(size: [u32; 2], views: &[SceneView<'_>]) -> Result<(), SceneError> {
    for (index, view) in views.iter().enumerate() {
        let [x, y, w, h] = view.viewport;
        let right = x.checked_add(w).ok_or(SceneError::InvalidGeometry)?;
        let bottom = y.checked_add(h).ok_or(SceneError::InvalidGeometry)?;
        if w == 0 || h == 0 || right > size[0] || bottom > size[1] {
            return Err(SceneError::InvalidGeometry);
        }
        for prior in &views[..index] {
            let [px, py, pw, ph] = prior.viewport;
            if x < px + pw && px < right && y < py + ph && py < bottom {
                return Err(SceneError::InvalidGeometry);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "scene/multi_view_tests.rs"]
mod multi_view_tests;

mod lod_geometry;
#[cfg(test)]
mod tests;
pub use lod_geometry::{SceneLodGeometry, SceneLodHistory};
mod skinning;
pub use skinning::{
    SceneSkinError, SceneSkinInstance, SceneSkinLodLevel, SceneSkinPose, SceneSkinSource,
    SceneSkinner,
};

#[cfg(test)]
mod memory_tests;

#[cfg(test)]
mod texture_memory_tests;

#[cfg(test)]
#[path = "scene/transform_memory_tests.rs"]
mod transform_memory_tests;

#[cfg(test)]
mod background_upload_regression;

#[cfg(test)]
mod opaque_partition_regression;
