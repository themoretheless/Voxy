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
use bytemuck::{Pod, Zeroable};
use glam::Mat4;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct SceneVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    /// Linear RGBA, with straight alpha.
    pub color: [f32; 4],
}

#[derive(Clone, Debug)]
pub struct SceneMesh {
    vertices: Vec<SceneVertex>,
    indices: Vec<u32>,
    // Private immutable buffers preserve the constructor's validation.
    validated: bool,
    material_coordinates: Option<Vec<[f32; 3]>>,
    authored_normals: Option<Vec<[f32; 3]>>,
    material_parameters: [f32; 4],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SceneError {
    InvalidGeometry,
    NonFiniteVertex,
    InvalidIndex,
    InvalidTexture,
    InvalidTransform,
    GeometryCapacityExceeded,
    MemoryBudget,
    MultisamplingNotEnabled,
    TimestampQueriesNotEnabled,
    DeviceMismatch,
}
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
        .allocate_buffers([crate::compute_memory::ManagedBufferDescriptor {
            label: "scene LOD indices",
            size: indices.len() as u64 * 4,
            contents: Some(bytemuck::cast_slice(indices)),
            usage: wgpu::BufferUsages::INDEX,
        }])
        .map_err(scene_memory_error)?;
    Ok(std::sync::Arc::new(buffer))
}
impl std::fmt::Display for SceneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "scene resource error: {self:?}")
    }
}
impl std::error::Error for SceneError {}

impl SceneMesh {
    #[must_use]
    pub fn vertices(&self) -> &[SceneVertex] {
        &self.vertices
    }
    #[must_use]
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }
    /// Explicit normal stream, including zero normals that request geometric
    /// flat shading in supporting scene shaders.
    #[must_use]
    pub fn authored_normals(&self) -> Option<&[[f32; 3]]> {
        self.authored_normals.as_deref()
    }
    /// Split a shader-tagged triangle layer, preserving vertex IDs, bind-space
    /// coordinates and optical parameters in both outputs. Returns None if absent.
    /// # Errors
    /// Rejects nonfinite tags, mixed-tag faces and a layer without an opaque substrate.
    // UV tags are exact shader identifiers, rather than measured coordinates.
    #[allow(clippy::float_cmp)]
    pub fn split_material_layer(
        &self,
        tag: f32,
        replacement: f32,
    ) -> Result<Option<(Self, Self)>, SceneError> {
        if !tag.is_finite() || !replacement.is_finite() {
            return Err(SceneError::NonFiniteVertex);
        }
        let mut layer = Vec::new();
        let mut opaque = Vec::new();
        for face in self.indices.chunks_exact(3) {
            let count = face
                .iter()
                .filter(|&&i| self.vertices[i as usize].uv[0] == tag)
                .count();
            match count {
                0 => opaque.extend_from_slice(face),
                3 => layer.extend_from_slice(face),
                _ => return Err(SceneError::InvalidGeometry),
            }
        }
        if layer.is_empty() {
            return Ok(None);
        }
        if opaque.is_empty() {
            return Err(SceneError::InvalidGeometry);
        }
        let mut body = self.clone();
        body.indices = opaque;
        let mut film = self.clone();
        film.indices = layer;
        for vertex in &mut film.vertices {
            if vertex.uv[0] == tag {
                vertex.uv[0] = replacement;
            }
        }
        Ok(Some((body, film)))
    }

    /// # Errors
    /// Rejects empty/non-triangular geometry, non-finite attributes and invalid indices.
    pub fn new(vertices: Vec<SceneVertex>, indices: Vec<u32>) -> Result<Self, SceneError> {
        Self::validate(&vertices, &indices)?;
        Ok(Self {
            vertices,
            indices,
            validated: true,
            material_coordinates: None,
            authored_normals: None,
            material_parameters: [1.333, 0.2, 0.08, 0.04],
        })
    }
    /// Preserve authored/deformed normals instead of regenerating welded
    /// geometry normals. Rejects mismatched counts and nonfinite values.
    pub fn with_normals(mut self, normals: Vec<[f32; 3]>) -> Result<Self, SceneError> {
        if normals.len() != self.vertices.len() {
            return Err(SceneError::InvalidGeometry);
        }
        if normals.iter().flatten().any(|value| !value.is_finite()) {
            return Err(SceneError::NonFiniteVertex);
        }
        self.authored_normals = Some(normals);
        Ok(self)
    }
    /// Four per-draw shader values supplied at vertex location 4. Existing
    /// shaders may ignore this stream. Rejects nonfinite values before mutation.
    /// # Errors
    /// Rejects nonfinite parameters with `SceneError::NonFiniteVertex`.
    pub fn with_material_parameters(mut self, values: [f32; 4]) -> Result<Self, SceneError> {
        if values.iter().any(|v| !v.is_finite()) {
            return Err(SceneError::NonFiniteVertex);
        }
        self.material_parameters = values;
        Ok(self)
    }
    /// Bind coordinates for procedural materials at vertex location 5.
    /// # Errors
    /// Rejects a mismatched count or nonfinite coordinates.
    pub fn with_material_coordinates(
        mut self,
        coordinates: Vec<[f32; 3]>,
    ) -> Result<Self, SceneError> {
        if coordinates.len() != self.vertices.len() {
            return Err(SceneError::InvalidGeometry);
        }
        if coordinates.iter().flatten().any(|v| !v.is_finite()) {
            return Err(SceneError::NonFiniteVertex);
        }
        self.material_coordinates = Some(coordinates);
        Ok(self)
    }
    /// Canonical procedural material coordinates, when explicitly provided.
    #[must_use]
    pub fn explicit_material_coordinates(&self) -> Option<&[[f32; 3]]> {
        self.material_coordinates.as_deref()
    }
    /// Prepares identical default normal and coordinate streams on a worker thread.
    /// This avoids geometric welding and allocations during native frame submission.
    #[must_use]
    pub fn with_prepared_upload_streams(mut self) -> Self {
        if self.authored_normals.is_none() { self.authored_normals = Some(smooth_normals(&self)); }
        if self.material_coordinates.is_none() {
            self.material_coordinates = Some(self.vertices.iter().map(|v| v.position).collect());
        }
        self
    }
    fn material_coordinates(&self) -> std::borrow::Cow<'_, [[f32; 3]]> {
        match &self.material_coordinates {
            Some(coordinates) => std::borrow::Cow::Borrowed(coordinates),
            None => std::borrow::Cow::Owned(self.vertices.iter().map(|v| v.position).collect()),
        }
    }
    fn validate_for_upload(&self) -> Result<(), SceneError> {
        if self.validated {
            Ok(())
        } else {
            Self::validate(&self.vertices, &self.indices)
        }
    }

    fn validate(vertices: &[SceneVertex], indices: &[u32]) -> Result<(), SceneError> {
        if vertices.is_empty()
            || indices.is_empty()
            || !indices.len().is_multiple_of(3)
            || u32::try_from(indices.len()).is_err()
        {
            return Err(SceneError::InvalidGeometry);
        }
        if vertices.iter().any(|v| {
            !(v.position[0].is_finite()
                && v.position[1].is_finite()
                && v.position[2].is_finite()
                && v.uv[0].is_finite()
                && v.uv[1].is_finite()
                && v.color[0].is_finite()
                && v.color[1].is_finite()
                && v.color[2].is_finite()
                && v.color[3].is_finite())
        }) {
            return Err(SceneError::NonFiniteVertex);
        }
        if indices
            .iter()
            .any(|&i| usize::try_from(i).map_or(true, |i| i >= vertices.len()))
        {
            return Err(SceneError::InvalidIndex);
        }
        Ok(())
    }

    /// Stable triangle-centroid ordering for straight-alpha geometry.
    /// Pass an affine model-to-view matrix, with the camera looking down -Z.
    /// This does not resolve intersecting triangles or sort separate meshes.
    /// # Errors
    /// Rejects non-affine, singular, nonfinite or overflowing transforms without mutation.
    pub fn sort_back_to_front(&mut self, local_to_view: Mat4) -> Result<(), SceneError> {
        if !local_to_view.is_finite()
            || local_to_view.row(3) != glam::Vec4::W
            || !local_to_view.determinant().is_finite()
            || local_to_view.determinant() == 0.0
        {
            return Err(SceneError::InvalidTransform);
        }
        let mut triangles = Vec::with_capacity(self.indices.len() / 3);
        for ids in self.indices.chunks_exact(3) {
            let depth = ids
                .iter()
                .map(|i| {
                    local_to_view
                        .transform_point3(glam::Vec3::from_array(
                            self.vertices[*i as usize].position,
                        ))
                        .z
                        / 3.0
                })
                .sum::<f32>();
            if !depth.is_finite() {
                return Err(SceneError::InvalidTransform);
            }
            triangles.push(([ids[0], ids[1], ids[2]], depth));
        }
        triangles.sort_by(|a, b| a.1.total_cmp(&b.1));
        for (destination, (ids, _)) in self.indices.chunks_exact_mut(3).zip(triangles) {
            destination.copy_from_slice(&ids);
        }
        Ok(())
    }

    /// Unit quad in XY, usable as a transformed sprite or 3D plane.
    #[must_use]
    pub fn quad(color: [f32; 4]) -> Self {
        Self {
            validated: color.iter().all(|component| component.is_finite()),
            material_coordinates: None,
            authored_normals: None,
            material_parameters: [1.333, 0.2, 0.08, 0.04],
            vertices: vec![
                SceneVertex {
                    position: [-0.5, -0.5, 0.0],
                    uv: [0.0, 1.0],
                    color,
                },
                SceneVertex {
                    position: [0.5, -0.5, 0.0],
                    uv: [1.0, 1.0],
                    color,
                },
                SceneVertex {
                    position: [0.5, 0.5, 0.0],
                    uv: [1.0, 0.0],
                    color,
                },
                SceneVertex {
                    position: [-0.5, 0.5, 0.0],
                    uv: [0.0, 0.0],
                    color,
                },
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
        }
    }
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
    opaque_shader: Option<(u64, wgpu::BindGroupLayout, wgpu::RenderPipeline, wgpu::RenderPipeline)>,
    partitioned_indices: bool,
    partition_cache: Vec<u32>,
}

/// Additional resource owners publish with their geometry after one admission.
struct UploadedGeometryBatch {
    geometries: Vec<SceneGeometry>,
    additional_buffers: Vec<std::sync::Arc<crate::ComputeStorage>>,
}

impl SceneGeometry {
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
    pub fn update_shared_vertex_streams(&mut self, queue: &wgpu::Queue, mesh: &SceneMesh) -> Result<(), SceneError> {
        self.update_mesh(queue, mesh, true)
    }

    fn update_mesh(&mut self, queue: &wgpu::Queue, mesh: &SceneMesh, preserve_partition: bool) -> Result<(), SceneError> {
        mesh.validate_for_upload()?;
        if mesh.vertices.len() > self.vertex_capacity || mesh.indices.len() > self.index_capacity {
            return Err(SceneError::GeometryCapacityExceeded);
        }
        let retain_partition = preserve_partition
            && self.partitioned_indices
            && self.normal_cache.indices == mesh.indices
            && self.normal_cache.positions.len() == mesh.vertices.len();
        let indices_changed = !retain_partition
            && (self.partitioned_indices || self.normal_cache.indices != mesh.indices);
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
        if self.normal_cache.refresh(mesh) {
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
    pub fn update_index_partition(&mut self, queue: &wgpu::Queue, indices: &[u32]) -> Result<(), SceneError> {
        self.update_index_partition_if_changed(queue, indices).map(|_| ())
    }

    /// Returns whether the index allocation changed. Exact validated cache hits
    /// retain the existing GPU stream; full-mesh updates invalidate this cache.
    pub fn update_index_partition_if_changed(&mut self, queue: &wgpu::Queue, indices: &[u32]) -> Result<bool, SceneError> {
        if self.partitioned_indices && self.partition_cache == indices {
            return Ok(false);
        }
        if indices.len() > self.index_capacity || indices.len() % 3 != 0
            || indices.iter().any(|i| *i as usize >= self.vertex_capacity) { return Err(SceneError::InvalidGeometry); }
        let count = u32::try_from(indices.len()).map_err(|_| SceneError::InvalidGeometry)?;
        if !indices.is_empty() { queue.write_buffer(&self.indices, 0, bytemuck::cast_slice(indices)); }
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
    texture: std::sync::Arc<crate::compute_memory::ManagedTexture>,
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
    /// Set the positive world half-space for planar_reflection_pbr_clip_shader.
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
    /// Set world transform, tint and clipping plane for PLANAR_REFLECTION_CLIP_SHADER.
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
    /// Set display MVP and reflected-camera MVP for PLANAR_REFLECTION_SURFACE_SHADER.
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
    pub fn upload_shared_mesh_partitions(&self, device: &wgpu::Device, mesh: &SceneMesh) -> Result<Vec<SceneGeometry>,SceneError> {
        self.upload_mesh_index_variants(device, mesh, &[mesh.indices(), mesh.indices()], wgpu::BufferUsages::empty())
    }
    pub async fn set_geometry_opaque_shader(&self, device: &wgpu::Device, geometry: &mut SceneGeometry, source: &str) -> Result<(),SceneShaderError> {
        if device != &self.device || device != &geometry.device {
            return Err(SceneShaderError("geometry material device mismatch".into()));
        }
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label:Some("geometry opaque shader"), source:wgpu::ShaderSource::Wgsl(source.into()) });
        let one = Self::create_shader_pipelines(device,self.color_format,&self.transform_layout,&self.texture_layout,&shader,1,
            self.shadow.as_ref().map(|s| &s.layout),self.environment.as_ref().map(|e| &e.layout));
        let four = Self::create_shader_pipelines(device,self.color_format,&self.transform_layout,&self.texture_layout,&shader,4,
            self.shadow.as_ref().map(|s| &s.layout),self.environment.as_ref().map(|e| &e.layout));
        if let Some(error) = scope.pop().await { return Err(SceneShaderError(error.to_string())); }
        geometry.opaque_shader = Some((self.shader_revision,self.transform_layout.clone(),one.0,four.0));
        Ok(())
    }
    pub fn upload_mesh(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
    ) -> Result<SceneGeometry, SceneError> {
        self.upload_mesh_with_usage(device, mesh, wgpu::BufferUsages::empty())
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
        additional_buffers: &[crate::compute_memory::ManagedBufferDescriptor<'_>],
    ) -> Result<UploadedGeometryBatch, SceneError> {
        use crate::compute_memory::ManagedBufferDescriptor;
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
        use crate::compute_memory::ManagedBufferDescriptor;
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
        texture: std::sync::Arc<crate::compute_memory::ManagedTexture>,
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
            .allocate_buffers([crate::compute_memory::ManagedBufferDescriptor {
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
                    let msaa = self.msaa_pipelines.as_ref().is_some_and(|p| std::ptr::eq(pipelines.0, &p.0));
                    pass.set_pipeline(draw.geometry.opaque_shader.as_ref().filter(|p| p.0 == self.shader_revision && p.1 == self.transform_layout).map_or(pipelines.0, |p| if msaa { &p.3 } else { &p.2 }));
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

#[derive(Debug, Default)]
struct NormalCache {
    authored: bool,
    positions: Vec<[f32; 3]>,
    indices: Vec<u32>,
    normals: Vec<[f32; 3]>,
}
impl NormalCache {
    fn from_mesh(mesh: &SceneMesh) -> Self {
        let mut cache = Self::default();
        cache.refresh(mesh);
        cache
    }
    // UV/colour updates do not change normals. Compare exact current geometry;
    // Geometry or authored normal changes refresh the stream; generated normals
    // rebuild the weld map only when no authored stream is supplied.
    #[allow(clippy::float_cmp)] // Exact geometry equality is the cache invalidation contract.
    fn refresh(&mut self, mesh: &SceneMesh) -> bool {
        let geometry_changed = self.indices != mesh.indices
            || self.positions.len() != mesh.vertices.len()
            || !self.positions.iter().zip(&mesh.vertices).all(|(p,v)| *p == v.position);
        let changed = match &mesh.authored_normals {
            Some(normals) => self.normals != *normals,
            None => self.authored || geometry_changed,
        };
        if changed {
            if let Some(normals) = &mesh.authored_normals {
                self.normals.clone_from(normals);
            } else {
                self.normals = smooth_normals(mesh);
            }
        }
        self.authored = mesh.authored_normals.is_some();
        if geometry_changed {
            self.positions.clear();
            self.positions.extend(mesh.vertices.iter().map(|v| v.position));
            self.indices.clone_from(&mesh.indices);
        }
        changed
    }
}

// Weld coincident OBJ seam vertices for area-weighted smooth shading. Rebuilt
// after every geometry update, including morphs and displaced liquid layers.
pub(crate) fn smooth_normals(mesh: &SceneMesh) -> Vec<[f32; 3]> {
    let mut lookup = std::collections::HashMap::with_capacity(mesh.vertices.len());
    let mut mapping = Vec::with_capacity(mesh.vertices.len());
    let mut sums = Vec::<glam::Vec3>::with_capacity(mesh.vertices.len());
    for vertex in &mesh.vertices {
        let key = vertex
            .position
            .map(|x| if x == 0. { 0 } else { x.to_bits() });
        // An injective 96-bit key avoids array hashing's length prefix while
        // retaining the randomized standard hasher and exact seam identity.
        let key = (u64::from(key[0]) << 32 | u64::from(key[1]), key[2]);
        let id = *lookup.entry(key).or_insert_with(|| {
            let id = sums.len();
            sums.push(glam::Vec3::ZERO);
            id
        });
        mapping.push(id);
    }
    for triangle in mesh.indices.chunks_exact(3) {
        let [a, b, c]: [u32; 3] = triangle.try_into().unwrap();
        let p = |i: u32| glam::Vec3::from_array(mesh.vertices[i as usize].position);
        let normal = (p(b) - p(a)).cross(p(c) - p(a));
        for i in [a, b, c] {
            sums[mapping[i as usize]] += normal;
        }
    }
    // Normalize each welded fan once, then scatter to render seam vertices.
    for sum in &mut sums {
        *sum = sum.normalize_or_zero();
    }
    mapping.iter().map(|&i| sums[i].to_array()).collect()
}

#[cfg(test)]
#[path = "scene/multi_view_tests.rs"]
mod multi_view_tests;

#[cfg(test)]
mod tests {
    #[test]
    fn disjoint_views_validate_regions_and_record_shared_geometry() {
        let empty = [];
        let regions = |rects: &[[u32; 4]]| {
            rects
                .iter()
                .map(|&viewport| SceneView {
                    viewport,
                    draws: &empty,
                })
                .collect::<Vec<_>>()
        };
        assert!(
            validate_view_regions([64, 32], &regions(&[[0, 0, 32, 32], [32, 0, 32, 32],])).is_ok()
        );
        for rects in [
            vec![[0, 0, 0, 32]],
            vec![[0, 0, 32, 0]],
            vec![[33, 0, 32, 32]],
            vec![[u32::MAX, 0, 2, 1]],
            vec![[0, 0, 33, 32], [32, 0, 32, 32]],
            vec![[0, 0, 64, 32], [1, 1, 1, 1]],
        ] {
            assert_eq!(
                validate_view_regions([64, 32], &regions(&rects)),
                Err(SceneError::InvalidGeometry)
            );
        }
        let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let geometry = renderer
            .upload_mesh(&device, &SceneMesh::quad([1.; 4]))
            .unwrap();
        let texture = renderer
            .upload_texture(&device, &queue, 1, 1, &[255; 4])
            .unwrap();
        let left_transform = renderer.create_transform(&device, Mat4::IDENTITY).unwrap();
        let right_transform = renderer
            .create_transform(&device, Mat4::from_scale(glam::Vec3::splat(0.5)))
            .unwrap();
        let left = [SceneDraw {
            geometry: &geometry,
            texture: &texture,
            transform: &left_transform,
            overlay: false,
        }];
        let right = [SceneDraw {
            geometry: &geometry,
            texture: &texture,
            transform: &right_transform,
            overlay: false,
        }];
        let target = |format| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("disjoint views test"),
                    size: wgpu::Extent3d {
                        width: 64,
                        height: 32,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        let color = target(wgpu::TextureFormat::Rgba8Unorm);
        let depth = target(wgpu::TextureFormat::Depth32Float);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        assert_eq!(
            renderer.encode_views(
                &mut encoder,
                &color,
                &depth,
                wgpu::Color::BLACK,
                &regions(&[[0, 0, 33, 32], [32, 0, 32, 32]])
            ),
            Err(SceneError::InvalidGeometry)
        );
        assert_eq!(
            renderer.encode_views_msaa4(
                &mut encoder,
                &color,
                &depth,
                &color,
                wgpu::Color::BLACK,
                &[]
            ),
            Err(SceneError::MultisamplingNotEnabled)
        );
        assert_eq!(
            renderer.encode_view_frame(
                &mut encoder,
                &color,
                &depth,
                wgpu::Color::BLACK,
                &[],
                &left
            ),
            Err(SceneError::InvalidGeometry)
        );
        renderer
            .encode_views(
                &mut encoder,
                &color,
                &depth,
                wgpu::Color::BLACK,
                &[
                    SceneView {
                        viewport: [0, 0, 32, 32],
                        draws: &left,
                    },
                    SceneView {
                        viewport: [32, 0, 32, 32],
                        draws: &right,
                    },
                ],
            )
            .unwrap();
        queue.submit([encoder.finish()]);
    }

    #[test]
    fn material_layer_split_preserves_binding_coordinates_and_parameters() {
        let vertices = (0..6)
            .map(|i| SceneVertex {
                position: [
                    if i % 3 == 1 { 1. } else { 0. },
                    if i % 3 == 2 { 1. } else { 0. },
                    (i / 3) as f32 * 0.001,
                ],
                uv: [if i < 3 { 0. } else { -2. }, 0.00001],
                color: [1.; 4],
            })
            .collect();
        let coordinates = vec![[0.123, 0.456, 0.789]; 6];
        let mesh = SceneMesh::new(vertices, vec![0, 1, 2, 3, 4, 5])
            .unwrap()
            .with_material_coordinates(coordinates.clone())
            .unwrap()
            .with_material_parameters([1.44, 0.3, 0.2, 0.1])
            .unwrap();
        let (opaque, film) = mesh.split_material_layer(-2., -6.).unwrap().unwrap();
        assert_eq!(opaque.indices(), &[0, 1, 2]);
        assert_eq!(film.indices(), &[3, 4, 5]);
        for output in [&opaque, &film] {
            assert_eq!(
                output.explicit_material_coordinates(),
                Some(coordinates.as_slice())
            );
            assert_eq!(output.material_parameters, mesh.material_parameters);
            assert_eq!(
                output
                    .vertices()
                    .iter()
                    .map(|v| v.position)
                    .collect::<Vec<_>>(),
                mesh.vertices()
                    .iter()
                    .map(|v| v.position)
                    .collect::<Vec<_>>()
            );
        }
        assert_eq!(mesh.vertices()[3].uv[0], -2.);
        assert_eq!(film.vertices()[3].uv[0], -6.);
    }
    #[test]
    fn material_layer_split_rejects_ambiguous_faces_and_clears_absent_layer() {
        let mut mesh = SceneMesh::quad([1.; 4]);
        assert!(mesh.split_material_layer(-2., -6.).unwrap().is_none());
        mesh.vertices[0].uv[0] = -2.;
        assert_eq!(
            mesh.split_material_layer(-2., -6.).unwrap_err(),
            SceneError::InvalidGeometry
        );
        assert_eq!(
            mesh.split_material_layer(f32::NAN, -6.).unwrap_err(),
            SceneError::NonFiniteVertex
        );
    }
    #[test]
    fn explicit_material_coordinates_survive_geometry_deformation() {
        let mut mesh = SceneMesh::quad([1.; 4]);
        let coordinates = mesh.material_coordinates().into_owned();
        assert!(
            mesh.clone()
                .with_material_coordinates(vec![[0.; 3]; 1])
                .is_err()
        );
        let mut invalid = coordinates.clone();
        invalid[0][0] = f32::NAN;
        assert!(mesh.clone().with_material_coordinates(invalid).is_err());
        mesh = mesh.with_material_coordinates(coordinates.clone()).unwrap();
        for v in &mut mesh.vertices {
            v.position[0] *= 1.5;
            v.position[1] += 0.2;
        }
        assert_eq!(mesh.material_coordinates().as_ref(), coordinates.as_slice());
        assert_ne!(
            mesh.vertices.iter().map(|v| v.position).collect::<Vec<_>>(),
            coordinates
        );
    }

    #[test]
    fn cancelling_and_degenerate_fans_have_finite_zero_normals() {
        let v = |position| super::SceneVertex {
            position,
            uv: [0.; 2],
            color: [1.; 4],
        };
        let mesh = super::SceneMesh::new(
            vec![v([0., 0., 0.]), v([1., 0., 0.]), v([0., 1., 0.])],
            vec![0, 1, 2, 0, 2, 1, 0, 0, 0],
        )
        .unwrap();
        assert!(super::smooth_normals(&mesh).iter().all(|n| *n == [0.; 3]));
    }
    #[test]
    fn smooth_normals_weld_seams_and_follow_deformation() {
        let v = |position| super::SceneVertex {
            position,
            uv: [0., 0.],
            color: [1.; 4],
        };
        let vertices = vec![
            v([0., 0., 0.]),
            v([1., 0., 0.]),
            v([0., 1., 0.]),
            v([-0., 0., 0.]),
            v([0., 1., 0.]),
            v([0., 0., 1.]),
        ];
        let mesh = super::SceneMesh::new(vertices.clone(), vec![0, 1, 2, 3, 4, 5]).unwrap();
        let mut cache = super::NormalCache::from_mesh(&mesh);
        assert!(!cache.refresh(&mesh));
        let mut recolored = vertices.clone();
        recolored[0].color = [0.2; 4];
        recolored[0].uv = [0.4, 0.8];
        let recolored = super::SceneMesh::new(recolored, vec![0, 1, 2, 3, 4, 5]).unwrap();
        assert!(!cache.refresh(&recolored));
        let normals = super::smooth_normals(&mesh);
        assert_eq!(normals[0], normals[3]);
        assert_eq!(normals[2], normals[4]);
        let expected = glam::Vec3::new(1., 0., 1.).normalize();
        assert!((glam::Vec3::from_array(normals[0]) - expected).length() < 1e-6);
        let mut changed = vertices;
        changed[5].position = [0., 0., 2.];
        let mesh = super::SceneMesh::new(changed, vec![0, 1, 2, 3, 4, 5]).unwrap();
        assert!(cache.refresh(&mesh));
        assert!(!cache.refresh(&mesh));
        let expected = glam::Vec3::new(2., 0., 1.).normalize();
        assert!(
            (glam::Vec3::from_array(super::smooth_normals(&mesh)[0]) - expected).length() < 1e-6
        );
    }
    use super::*;

    #[test]
    fn mesh_admission_rejects_every_nonfinite_vertex_lane() {
        let base = SceneVertex {
            position: [0.; 3],
            uv: [0.; 2],
            color: [1.; 4],
        };
        for lane in 0..9 {
            for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                let mut v = base;
                match lane {
                    0..=2 => v.position[lane] = value,
                    3..=4 => v.uv[lane - 3] = value,
                    _ => v.color[lane - 5] = value,
                }
                assert!(matches!(
                    SceneMesh::new(vec![v; 3], vec![0, 1, 2]),
                    Err(SceneError::NonFiniteVertex)
                ));
            }
        }
        let v = SceneVertex {
            position: [f32::from_bits(1), -0., f32::MAX],
            uv: [0.; 2],
            color: [1.; 4],
        };
        assert!(SceneMesh::new(vec![v; 3], vec![0, 1, 2]).is_ok());
    }

    #[test]
    fn rejects_malformed_mesh_and_non_finite_helper_input() {
        let valid = SceneMesh::quad([1.0; 4]);
        assert_eq!(
            SceneMesh::new(valid.vertices.clone(), vec![0, 1, 9]).unwrap_err(),
            SceneError::InvalidIndex
        );
        assert_eq!(
            SceneMesh::new(valid.vertices.clone(), vec![0, 1]).unwrap_err(),
            SceneError::InvalidGeometry
        );
        let invalid = SceneMesh::quad([f32::NAN; 4]);
        assert_eq!(
            SceneMesh::validate(&invalid.vertices, &invalid.indices),
            Err(SceneError::NonFiniteVertex)
        );
    }

    #[test]
    fn alpha_sort_tracks_camera_and_rejects_bad_transforms_atomically() {
        let near = SceneMesh::quad([1.0; 4]);
        let mut vertices = near.vertices.clone();
        vertices.extend(near.vertices.iter().map(|v| SceneVertex {
            position: [v.position[0], v.position[1], -2.0],
            ..*v
        }));
        let mut mesh = SceneMesh::new(vertices, vec![0, 1, 2, 4, 5, 6]).unwrap();
        mesh.sort_back_to_front(Mat4::IDENTITY).unwrap();
        assert_eq!(mesh.indices, [4, 5, 6, 0, 1, 2]);
        mesh.sort_back_to_front(Mat4::from_rotation_y(std::f32::consts::PI))
            .unwrap();
        assert_eq!(mesh.indices, [0, 1, 2, 4, 5, 6]);
        let before = mesh.indices.clone();
        for invalid in [
            Mat4::ZERO,
            Mat4::from_cols_array(&[f32::NAN; 16]),
            Mat4::from_cols(
                glam::Vec4::new(1.0, 0.0, 0.0, 1.0),
                glam::Vec4::Y,
                glam::Vec4::Z,
                glam::Vec4::W,
            ),
        ] {
            assert_eq!(
                mesh.sort_back_to_front(invalid),
                Err(SceneError::InvalidTransform)
            );
            assert_eq!(mesh.indices, before);
        }
    }

    #[test]
    fn resource_validation_precedes_gpu_upload() {
        let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        for wrap in [TextureWrap::Clamp, TextureWrap::Repeat, TextureWrap::Mirror] {
            for filter in [TextureFilter::Nearest, TextureFilter::Linear] {
                renderer
                    .upload_texture_with_sampling(
                        &device,
                        &queue,
                        1,
                        1,
                        &[255; 4],
                        TextureSampling {
                            wrap_u: wrap,
                            wrap_v: wrap,
                            min_filter: filter,
                            mag_filter: filter,
                            ..TextureSampling::default()
                        },
                    )
                    .unwrap();
            }
        }
        assert!(matches!(
            renderer.upload_texture_with_sampling(
                &device,
                &queue,
                1,
                1,
                &[255; 4],
                TextureSampling {
                    anisotropy: 2,
                    mipmap_filter: Some(TextureFilter::Nearest),
                    ..TextureSampling::default()
                },
            ),
            Err(SceneError::InvalidTexture)
        ));
        let image = renderer
            .upload_texture(&device, &queue, 1, 1, &[255; 4])
            .unwrap();
        let rebound = renderer
            .texture_binding(
                &device,
                &image,
                TextureSampling {
                    min_filter: TextureFilter::Nearest,
                    ..TextureSampling::default()
                },
                1,
            )
            .unwrap();
        assert_eq!(image.texture(), rebound.texture());
        for count in [0, 2] {
            assert!(matches!(
                renderer.texture_binding(&device, &image, TextureSampling::default(), count),
                Err(SceneError::InvalidTexture)
            ));
        }
        let (foreign, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        assert!(matches!(
            renderer.texture_binding(&foreign, &image, TextureSampling::default(), 1),
            Err(SceneError::DeviceMismatch)
        ));
        for (vertices, indices) in [(0, 3), (4, 0), (usize::MAX, 3), (4, usize::MAX)] {
            assert!(matches!(
                renderer.reserve_geometry(&device, vertices, indices),
                Err(SceneError::GeometryCapacityExceeded)
            ));
        }
        let mut reserved = renderer.reserve_geometry(&device, 8, 12).unwrap();
        assert_eq!(reserved.capacity(), (8, 12));
        assert_eq!(reserved.index_count, 0);
        let mesh = SceneMesh::quad([1.0; 4]);
        let mut geometry = renderer.upload_mesh(&device, &mesh).unwrap();
        let triangle = SceneMesh::new(mesh.vertices.clone(), vec![0, 1, 2]).unwrap();
        reserved.update(&queue, &triangle).unwrap();
        reserved.clear();
        reserved.update(&queue, &mesh).unwrap();
        assert_eq!(reserved.index_count(), 6);
        geometry.update(&queue, &triangle).unwrap();
        assert_eq!(geometry.index_count(), 3);
        let oversized =
            SceneMesh::new(mesh.vertices.clone(), vec![0, 1, 2, 0, 2, 3, 0, 1, 2]).unwrap();
        assert_eq!(
            geometry.update(&queue, &oversized),
            Err(SceneError::GeometryCapacityExceeded)
        );
        assert_eq!(geometry.index_count(), 3);
        assert_eq!(
            geometry.update(&queue, &SceneMesh::quad([f32::NAN; 4])),
            Err(SceneError::NonFiniteVertex)
        );
        assert_eq!(geometry.index_count(), 3);
        for (width, height, rgba) in [
            (0, 1, &[][..]),
            (1, 1, &[255; 3][..]),
            (u32::MAX, u32::MAX, &[][..]),
        ] {
            assert!(matches!(
                renderer.upload_texture(&device, &queue, width, height, rgba),
                Err(SceneError::InvalidTexture)
            ));
        }
        assert!(matches!(
            renderer.create_transform(&device, Mat4::from_cols_array(&[f32::NAN; 16])),
            Err(SceneError::InvalidTransform)
        ));
        assert!(pollster::block_on(scope.pop()).is_none());
    }
}
mod lod_geometry;
pub use lod_geometry::{SceneLodGeometry, SceneLodHistory};
mod skinning;
pub use skinning::{
    SceneSkinError, SceneSkinInstance, SceneSkinLodLevel, SceneSkinPose, SceneSkinSource,
    SceneSkinner,
};

#[cfg(test)]
mod memory_tests {
    use super::*;
    use crate::{ComputeError, ComputeMemoryBudget, LodIndexSet};

    fn read_vertices(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        geometry: &SceneGeometry,
    ) -> Vec<u8> {
        let size = geometry.vertices.size();
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("budget regression readback"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&geometry.vertices, 0, &readback, 0, size);
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        rx.recv().unwrap().unwrap();
        let data = readback.slice(..).get_mapped_range().unwrap().to_vec();
        readback.unmap();
        data
    }

    #[test]
    #[ignore = "requires physical GPU; shared mesh budget and retirement qualification"]
    fn gpu_shared_mesh_budget_preserves_replacement_and_lod_owners() {
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
            .expect("physical GPU required for mesh budget qualification");
        eprintln!("mesh budget adapter: {:?}", adapter.get_info());
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mesh = SceneMesh::quad([1.; 4]);
        let bytes = SceneRenderer::mesh_allocation_bytes(&mesh);
        let variants = LodIndexSet::new(
            mesh.vertices.len(),
            vec![(0., mesh.indices.clone()), (0.01, mesh.indices.clone())],
        )
        .unwrap();
        let bundle_bytes = SceneRenderer::lod_mesh_allocation_bytes(&mesh, &variants).unwrap();
        let budget = ComputeMemoryBudget::configure(&device, bundle_bytes + 20).unwrap();
        let compute = budget
            .allocate_storage("shared compute owner", &[0; 4])
            .unwrap();
        let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        // Extra copy usage only for independent GPU-byte verification.
        let mut geometry = renderer
            .upload_mesh_with_usage(&device, &mesh, wgpu::BufferUsages::COPY_SRC)
            .unwrap();
        geometry.set_depth_mode(SceneDepthMode::Xray);
        assert_eq!(budget.stats().allocated_bytes, bytes + 4);
        let expected = bytemuck::cast_slice::<SceneVertex, u8>(&mesh.vertices).to_vec();
        assert_eq!(read_vertices(&device, &queue, &geometry), expected);
        let before = budget.stats();
        assert_eq!(
            renderer.replace_mesh(&device, &mut geometry, &mesh),
            Err(SceneError::MemoryBudget)
        );
        assert_eq!(geometry.depth_mode(), SceneDepthMode::Xray);
        assert_eq!(budget.stats(), before);
        assert_eq!(read_vertices(&device, &queue, &geometry), expected);
        // Enough room for early individual buffers, but not a complete bundle:
        // failed upload must neither allocate nor retire a partial candidate.
        assert!(matches!(
            renderer.upload_lod_mesh(&device, &mesh, &variants),
            Err(SceneError::MemoryBudget)
        ));
        assert!(matches!(
            renderer.reserve_geometry(&device, mesh.vertices.len(), mesh.indices.len()),
            Err(SceneError::MemoryBudget)
        ));
        assert_eq!(budget.stats(), before);
        let spare = budget
            .allocate_storage("no partial budget leak", &[0; 16])
            .unwrap();
        drop(spare);
        drop(compute);
        drop(geometry);
        assert_eq!(budget.stats().allocated_bytes, bytes + 20);
        budget.discard_retired().unwrap();
        assert_eq!(budget.stats().allocated_bytes, 0);

        let mut bundle = renderer.upload_lod_mesh(&device, &mesh, &variants).unwrap();
        assert_eq!(budget.stats().allocated_bytes, bundle_bytes);
        assert_eq!(budget.stats().allocated_buffers, 6); // Four shared streams, two indices.
        let vertex_owner = bundle.level(1).unwrap().vertices.clone();
        let shared_bytes = vertex_owner.size();
        assert_eq!(
            renderer.replace_lod_mesh(&device, &mut bundle, &mesh, &variants),
            Err(SceneError::MemoryBudget)
        );
        assert_eq!(budget.stats().allocated_bytes, bundle_bytes);
        assert_eq!(budget.stats().retired_buffers, 0);
        drop(bundle);
        assert_eq!(budget.stats().retired_buffers, 5);
        budget.discard_retired().unwrap();
        assert_eq!(budget.stats().allocated_bytes, shared_bytes);
        assert_eq!(budget.stats().allocated_buffers, 1);
        drop(vertex_owner);
        assert_eq!(budget.stats().retired_buffers, 1);
        budget.discard_retired().unwrap();
        assert_eq!(budget.stats().allocated_bytes, 0);
        let reserved = renderer
            .reserve_geometry(&device, mesh.vertices.len(), mesh.indices.len())
            .unwrap();
        assert_eq!(budget.stats().allocated_bytes, bytes);
        assert_eq!(reserved.index_count, 0);
        drop(reserved);
        budget.discard_retired().unwrap();
        assert_eq!(budget.stats().allocated_bytes, 0);
        assert!(pollster::block_on(scope.pop()).is_none());
        assert!(matches!(
            budget.allocate_storage("exhaustion", &vec![0; (bundle_bytes + 24) as usize]),
            Err(ComputeError::MemoryBudget)
        ));
    }
}

#[cfg(test)]
mod texture_memory_tests {
    use super::*;
    #[test]
    #[ignore = "requires physical GPU; shared colour mip/HDR admission and lifetime"]
    fn gpu_image_hdr_budget_shares_views_and_preserves_uploaded_mips() {
        let gpu = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(gpu.request_adapter(&Default::default())).unwrap();
        eprintln!("texture budget adapter: {:?}", adapter.get_info());
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mesh = SceneMesh::quad([1.; 4]);
        let mesh_bytes = SceneRenderer::mesh_allocation_bytes(&mesh);
        let budget =
            crate::ComputeMemoryBudget::configure(&device, mesh_bytes + 44 + 56 + 4).unwrap();
        let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let geometry = renderer.upload_mesh(&device, &mesh).unwrap();
        let compute = budget.allocate_storage("shared compute", &[0; 4]).unwrap();
        let base = [255, 0, 0, 255].repeat(8);
        let middle = [0, 255, 0, 255].repeat(2);
        let tail = [0, 0, 255, 255];
        let image = renderer
            .upload_texture_levels(
                &device,
                &queue,
                &[(4, 2, &base), (2, 1, &middle), (1, 1, &tail)],
                TextureSampling::default(),
            )
            .unwrap();
        assert_eq!(image.allocation_bytes(), 44);
        let alias = renderer
            .texture_binding(&device, &image, TextureSampling::default(), 1)
            .unwrap();
        let before = budget.stats();
        assert_eq!(before.allocated_textures, 1);
        assert!(matches!(
            renderer.texture_binding(&device, &image, TextureSampling::default(), 4),
            Err(SceneError::InvalidTexture)
        ));
        assert_eq!(budget.stats(), before);
        let hdr = crate::HdrMipPyramid::new(&device, 3, 2).unwrap();
        assert_eq!(hdr.allocation_bytes(), 56);
        let hdr_alias = renderer
            .bind_image(
                &device,
                hdr.managed_texture(),
                TextureSampling::default(),
                2,
            )
            .unwrap();
        assert_eq!(hdr_alias.allocation_bytes(), 56);
        assert_eq!(budget.stats().allocated_bytes, mesh_bytes + 104);
        assert_eq!(budget.stats().allocated_textures, 2);
        assert_eq!(budget.stats().allocated_buffers, 6);
        let full = budget.stats();
        assert!(matches!(
            renderer.upload_texture(&device, &queue, 1, 1, &[255; 4]),
            Err(SceneError::MemoryBudget)
        ));
        assert!(matches!(
            renderer.create_sampled_color(&device, 1, 1),
            Err(SceneError::MemoryBudget)
        ));
        assert!(matches!(
            crate::HdrMipPyramid::new(&device, 1, 1),
            Err(crate::RendererError::Scene(SceneError::MemoryBudget))
        ));
        assert_eq!(budget.stats(), full);

        // Independently read the GPU's three uploaded sRGB mip texels after
        // failed admission. End-point primary colours decode exactly to 0/1.
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mip budget readback"), source: wgpu::ShaderSource::Wgsl(r#"
                @group(0) @binding(0) var image: texture_2d<f32>;
                @group(0) @binding(1) var<storage, read_write> colors: array<vec4<f32>, 3>;
                @compute @workgroup_size(1) fn main() {
                    for (var i = 0u; i < 3u; i++) { colors[i] = textureLoad(image, vec2<i32>(0), i32(i)); }
                }
            "#.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let result = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 48,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 48,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let view = image.texture().create_view(&Default::default());
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: result.as_entire_binding(),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &binding, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&result, 0, &staging, 0, 48);
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        staging.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            tx.send(r).unwrap();
        });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        rx.recv().unwrap().unwrap();
        let mapped = staging.slice(..).get_mapped_range().unwrap();
        assert_eq!(
            bytemuck::cast_slice::<u8, [f32; 4]>(&mapped),
            &[[1., 0., 0., 1.], [0., 1., 0., 1.], [0., 0., 1., 1.]]
        );
        drop(mapped);
        staging.unmap();
        drop((view, binding, pipeline));
        drop(image);
        drop(hdr);
        assert_eq!(budget.stats().retired_textures, 0);
        drop(alias);
        drop(hdr_alias);
        drop(geometry);
        drop(compute);
        assert_eq!(budget.stats().retired_textures, 2);
        assert_eq!(budget.stats().retired_buffers, 6);
        let mut ticket = budget.begin_retirement(&queue);
        assert_eq!(budget.stats().allocated_bytes, mesh_bytes + 104);
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        assert!(ticket.try_finish().unwrap());
        assert!(ticket.try_finish().unwrap());
        assert_eq!(budget.stats(), crate::ComputeMemoryStats::default());
        let replacement = renderer.create_sampled_color(&device, 1, 1).unwrap();
        assert_eq!(replacement.allocation_bytes(), 4);
        drop(replacement);
        budget.discard_retired().unwrap();
        assert_eq!(budget.stats(), crate::ComputeMemoryStats::default());
        assert!(pollster::block_on(scope.pop()).is_none());
        eprintln!(
            "TEXTURE_DEVICE_BUDGET_PASS image_mips_bytes=44 hdr_npot_mips_bytes=56 views_share_owner=true uploaded_mips_survive_rejection=true final_charged_bytes=0"
        );
    }
}

#[cfg(test)]
#[path = "scene/transform_memory_tests.rs"]
mod transform_memory_tests;

#[cfg(test)]
mod background_upload_regression {
    use super::*;
    #[test]
    fn authored_normal_cache_tracks_geometry_without_duplicate_streams() {
        let mut mesh = SceneMesh::quad([1.;4]).with_normals(vec![[0.,0.,1.];4]).unwrap();
        let mut cache = NormalCache::from_mesh(&mesh);
        let allocation = cache.normals.as_ptr();
        mesh.vertices[2].position[2] = 0.25;
        assert!(!cache.refresh(&mesh), "unchanged authored normals need no GPU upload");
        assert_eq!(cache.normals.as_ptr(), allocation);
        assert_eq!(cache.positions[2], mesh.vertices[2].position);
        mesh.authored_normals.as_mut().unwrap()[2] = [0.,1.,0.];
        assert!(cache.refresh(&mesh));
        assert_eq!(cache.normals, *mesh.authored_normals.as_ref().unwrap());
        mesh.authored_normals = None;
        assert!(cache.refresh(&mesh), "dropping authored normals regenerates from current geometry");
        assert_eq!(cache.normals, smooth_normals(&mesh));
        assert!(!cache.refresh(&mesh));
    }
    #[test]
    fn packed_weld_keys_preserve_seams_and_signed_zero() {
        let mut mesh = SceneMesh::quad([1.;4]);
        mesh.vertices[2].position[2] = 0.15;
        let mut duplicate = mesh.vertices.clone();
        for v in &mut duplicate {
            if v.position[2] == 0.0 { v.position[2] = -0.0; }
        }
        mesh.vertices.extend(duplicate);
        mesh.indices.extend([4,5,6,4,6,7]);
        let mut lookup = std::collections::HashMap::new();
        let mut mapping = Vec::new();
        let mut sums = Vec::<glam::Vec3>::new();
        for vertex in &mesh.vertices {
            let key = vertex.position.map(|x| if x == 0. {0} else {x.to_bits()});
            let id = *lookup.entry(key).or_insert_with(|| {
                let id = sums.len(); sums.push(glam::Vec3::ZERO); id
            });
            mapping.push(id);
        }
        for tri in mesh.indices.chunks_exact(3) {
            let p = |i: u32| glam::Vec3::from_array(mesh.vertices[i as usize].position);
            let n = (p(tri[1])-p(tri[0])).cross(p(tri[2])-p(tri[0]));
            for &i in tri { sums[mapping[i as usize]] += n; }
        }
        for n in &mut sums { *n = n.normalize_or_zero(); }
        let reference: Vec<_> = mapping.iter().map(|&i| sums[i].to_array()).collect();
        assert_eq!(smooth_normals(&mesh), reference);
    }
    #[test]
    fn worker_preparation_preserves_default_welded_shading_and_authored_streams() {
        let mut mesh = SceneMesh::quad([0.7,0.4,0.2,1.]);
        mesh.vertices[2].position[2] = 0.3;
        let fallback = NormalCache::from_mesh(&mesh);
        let coordinates = mesh.material_coordinates().into_owned();
        let ready = mesh.clone().with_prepared_upload_streams();
        let prepared = NormalCache::from_mesh(&ready);
        assert_eq!(fallback.normals, prepared.normals);
        assert_eq!(coordinates, ready.material_coordinates().as_ref());
        assert_eq!(mesh.indices, ready.indices);
        let normals = vec![[0.,0.,1.]; mesh.vertices.len()];
        let authored = mesh.with_normals(normals.clone()).unwrap().with_material_coordinates(coordinates.clone()).unwrap().with_prepared_upload_streams();
        assert_eq!(authored.authored_normals.as_ref(), Some(&normals));
        assert_eq!(authored.material_coordinates.as_ref(), Some(&coordinates));
    }
}

#[cfg(test)]
mod opaque_partition_regression {
    use super::*;
    #[test]
    fn shared_partitions_keep_vertex_owners_and_reject_invalid_replacement() {
        let (device,queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let scene = SceneRenderer::new(&device,wgpu::TextureFormat::Rgba8Unorm);
        let mesh = SceneMesh::quad([1.;4]);
        let mut parts = scene.upload_shared_mesh_partitions(&device,&mesh).unwrap();
        assert!(std::sync::Arc::ptr_eq(&parts[0].vertices,&parts[1].vertices));
        assert!(std::sync::Arc::ptr_eq(&parts[0].normals,&parts[1].normals));
        assert!(parts[0].update_index_partition_if_changed(&queue,&mesh.indices()[..3]).unwrap());
        parts[1].update_index_partition(&queue,&mesh.indices()[3..]).unwrap();
        let cached = parts[0].partition_cache.as_ptr();
        assert!(!parts[0].update_index_partition_if_changed(&queue,&mesh.indices()[..3]).unwrap());
        assert_eq!(parts[0].partition_cache.as_ptr(), cached);
        assert_eq!(parts[0].partition_cache, mesh.indices()[..3]);
        assert_eq!(parts[0].index_count(),3);
        assert_eq!(parts[1].index_count(),3);
        let mut deformed = mesh.clone();
        deformed.vertices[0].position[2] = 0.1;
        parts[0].update_shared_vertex_streams(&queue, &deformed).unwrap();
        assert_eq!(parts[0].index_count(), 3);
        assert!(parts[0].partitioned_indices);
        assert_eq!(parts[0].partition_cache, mesh.indices()[..3]);
        deformed.indices.truncate(3);
        parts[0].update_shared_vertex_streams(&queue, &deformed).unwrap();
        assert_eq!(parts[0].index_count(), 3);
        assert!(!parts[0].partitioned_indices);
        assert!(parts[0].partition_cache.is_empty());
        parts[0].update_index_partition(&queue, &mesh.indices()[..3]).unwrap();
        parts[0].update(&queue, &mesh).unwrap();
        assert_eq!(parts[0].index_count(), 6);
        assert!(parts[0].update_index_partition(&queue,&[0,1,999]).is_err());
        assert_eq!(parts[0].index_count(),6);
        let invalid = pollster::block_on(scene.set_geometry_opaque_shader(&device,&mut parts[0],"invalid wgsl"));
        assert!(invalid.is_err());
        assert!(parts[0].opaque_shader.is_none());
    }
}
