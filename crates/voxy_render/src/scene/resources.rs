use super::geometry::{NormalCache, SceneError, SceneMesh, SceneVertex};
use glam::Mat4;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TextureWrap {
    #[default]
    Clamp,
    Repeat,
    Mirror,
}

impl TextureWrap {
    pub(super) fn address_mode(self) -> wgpu::AddressMode {
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
    pub(super) fn filter_mode(self) -> wgpu::FilterMode {
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
    pub(super) fn descriptor(self) -> wgpu::SamplerDescriptor<'static> {
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

pub(super) fn scene_memory_error(error: crate::ComputeError) -> SceneError {
    match error {
        crate::ComputeError::MemoryBudget => SceneError::MemoryBudget,
        _ => SceneError::InvalidGeometry,
    }
}

pub(super) fn managed_scene_indices(
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
    /// Hardware early-Z friendly closed opaque geometry (`Face::Back` culling, no blend/discard).
    SolidOpaque,
    #[default]
    Opaque,
    /// Tests world depth without writing it. Supply triangles back to front.
    Transparent,
    /// Reveals selected geometry through occluders without modifying world depth.
    Xray,
}

#[derive(Debug)]
pub struct SceneGeometry {
    pub(super) device: wgpu::Device,
    #[doc(hidden)]
    pub vertices: std::sync::Arc<crate::ComputeStorage>,
    #[doc(hidden)]
    pub normals: std::sync::Arc<crate::ComputeStorage>,
    pub(super) normal_cache: NormalCache,
    pub(super) tangents: Option<std::sync::Arc<crate::ComputeStorage>>,
    pub material_coordinates: std::sync::Arc<crate::ComputeStorage>,
    pub(super) coordinate_cache: Vec<[f32; 3]>,
    #[doc(hidden)]
    pub material_parameters: std::sync::Arc<crate::ComputeStorage>,
    #[doc(hidden)]
    pub indices: std::sync::Arc<crate::ComputeStorage>,
    pub(super) index_count: u32,
    pub(super) vertex_capacity: usize,
    pub(super) index_capacity: usize,
    pub(super) depth_mode: SceneDepthMode,
    pub(super) opaque_shader: Option<(
        u64,
        wgpu::BindGroupLayout,
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
    )>,
    pub(super) partitioned_indices: bool,
    pub(super) partition_cache: Vec<u32>,
    /// Process-unique identity so shadow consumers can recognise the same allocation.
    pub(super) identity: u64,
    /// Bumped on every GPU write of vertices or indices; part of the shadow key.
    pub(super) revision: u64,
}

static NEXT_GEOMETRY_IDENTITY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub(crate) fn next_geometry_identity() -> u64 {
    NEXT_GEOMETRY_IDENTITY.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// Additional resource owners publish with their geometry after one admission.
pub(super) struct UploadedGeometryBatch {
    pub(super) geometries: Vec<SceneGeometry>,
    pub(super) additional_buffers: Vec<std::sync::Arc<crate::ComputeStorage>>,
}

impl SceneGeometry {
    pub(crate) fn deformation_vertex_capacity(&self) -> usize {
        self.vertex_capacity
    }
    pub(crate) fn deformation_normals(&self) -> &wgpu::Buffer {
        &self.normals
    }
    pub(crate) fn deformation_tangents(&self) -> Option<&wgpu::Buffer> {
        self.tangents.as_deref().map(|b| &**b)
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

    /// Binds all vertex streams and the index buffer for this geometry in a single fused call.
    #[inline]
    pub(crate) fn bind_vertex_and_index_streams(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_vertex_buffer(1, self.normals.slice(..));
        pass.set_vertex_buffer(2, self.material_parameters.slice(..));
        pass.set_vertex_buffer(3, self.material_coordinates.slice(..));
        if let Some(tangents) = &self.tangents {
            pass.set_vertex_buffer(4, tangents.slice(..));
        }
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
    }

    /// Binds per-vertex streams (`0`, `1`, `3`, and optional `4`) and the index buffer for
    /// multi-instance rendering (`0..instance_count`).
    #[inline]
    pub(crate) fn bind_instanced_streams(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_vertex_buffer(1, self.normals.slice(..));
        pass.set_vertex_buffer(3, self.material_coordinates.slice(..));
        if let Some(tangents) = &self.tangents {
            pass.set_vertex_buffer(4, tangents.slice(..));
        }
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
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
        self.update_mesh(queue, mesh, false, 0)
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
        self.update_mesh(queue, mesh, true, 0)
    }

    /// Update CPU-owned appended details while a compute owner replaces the base
    /// vertex/normal prefix. Requires explicit normals and compute-writable streams.
    pub fn update_after_deforming_prefix(
        &mut self,
        queue: &wgpu::Queue,
        mesh: &SceneMesh,
        prefix: usize,
    ) -> Result<(), SceneError> {
        self.update_mesh(queue, mesh, false, prefix)
    }

    fn update_mesh(
        &mut self,
        queue: &wgpu::Queue,
        mesh: &SceneMesh,
        preserve_partition: bool,
        prefix: usize,
    ) -> Result<(), SceneError> {
        mesh.validate_for_upload()?;
        if mesh.vertices.len() > self.vertex_capacity || mesh.indices.len() > self.index_capacity {
            return Err(SceneError::GeometryCapacityExceeded);
        }
        if prefix > mesh.vertices.len()
            || (prefix > 0
                && (mesh.authored_normals().is_none()
                    || !self.vertices.usage().contains(wgpu::BufferUsages::STORAGE)
                    || !self.normals.usage().contains(wgpu::BufferUsages::STORAGE)))
        {
            return Err(SceneError::InvalidGeometry);
        }
        // OPTIMIZATION #41: Mark shadows dirty on geometry update
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
        if prefix < mesh.vertices.len() {
            queue.write_buffer(
                &self.vertices,
                (prefix * 36) as u64,
                bytemuck::cast_slice(&mesh.vertices[prefix..]),
            );
        }
        let normals_changed = self.normal_cache.refresh_with(mesh, indices_equal);
        if (normals_changed || self.normal_cache.gpu_prefix_dirty) && prefix < mesh.vertices.len() {
            queue.write_buffer(
                &self.normals,
                (prefix * 12) as u64,
                bytemuck::cast_slice(&self.normal_cache.normals[prefix..]),
            );
        }
        self.normal_cache.gpu_prefix_dirty = prefix > 0;
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
            + self.tangents.as_ref().map_or(0, |b| b.size())
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

pub(super) fn geometry_sizes(
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
    pub(super) device: wgpu::Device,
    pub(super) bind_group: wgpu::BindGroup,
    pub(super) texture: std::sync::Arc<voxy_gpu_core::ManagedTexture>,
    pub(super) sampler: wgpu::Sampler,
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
    pub(super) buffer: crate::ComputeStorage,
    pub(super) bind_group: wgpu::BindGroup,
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

impl super::SceneRenderer {
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

    pub(super) fn upload_texture_levels(
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
        let mvp_cols = mvp.to_cols_array();
        let id_cols = Mat4::IDENTITY.to_cols_array();
        let mut uniform = [0.0_f32; 64];
        uniform[0..16].copy_from_slice(&mvp_cols);
        uniform[16..32].copy_from_slice(&mvp_cols);
        uniform[32..36].copy_from_slice(&[0., 0., 2., 1.]);
        uniform[36..52].copy_from_slice(&id_cols);
        uniform[52..56].copy_from_slice(&[1.; 4]);
        uniform[56..60].copy_from_slice(&[0., 0., 1., 0.]);
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
}
