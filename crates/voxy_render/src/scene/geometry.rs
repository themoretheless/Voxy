//! CPU-side scene geometry: mesh data, validation, welded smooth normals.
//! No `wgpu` here; upload and residency live in the parent module.

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
    pub(super) vertices: Vec<SceneVertex>,
    pub(super) indices: Vec<u32>,
    // Private immutable buffers preserve the constructor's validation.
    pub(super) validated: bool,
    pub(super) material_coordinates: Option<Vec<[f32; 3]>>,
    pub(super) authored_normals: Option<Vec<[f32; 3]>>,
    pub(super) material_parameters: [f32; 4],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("scene resource error: {self:?}")]
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
        if self.authored_normals.is_none() {
            self.authored_normals = Some(smooth_normals(&self));
        }
        if self.material_coordinates.is_none() {
            self.material_coordinates = Some(self.vertices.iter().map(|v| v.position).collect());
        }
        self
    }
    pub(super) fn material_coordinates(&self) -> std::borrow::Cow<'_, [[f32; 3]]> {
        match &self.material_coordinates {
            Some(coordinates) => std::borrow::Cow::Borrowed(coordinates),
            None => std::borrow::Cow::Owned(self.vertices.iter().map(|v| v.position).collect()),
        }
    }
    pub(super) fn validate_for_upload(&self) -> Result<(), SceneError> {
        if self.validated {
            Ok(())
        } else {
            Self::validate(&self.vertices, &self.indices)
        }
    }

    pub(super) fn validate(vertices: &[SceneVertex], indices: &[u32]) -> Result<(), SceneError> {
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

#[derive(Debug, Default)]
#[doc(hidden)]
pub struct NormalCache {
    pub(super) gpu_prefix_dirty: bool,
    pub(super) authored: bool,
    pub(super) positions: Vec<[f32; 3]>,
    pub(super) indices: Vec<u32>,
    #[doc(hidden)]
    pub normals: Vec<[f32; 3]>,
}
impl NormalCache {
    #[doc(hidden)]
    pub fn from_mesh(mesh: &SceneMesh) -> Self {
        let mut cache = Self::default();
        cache.refresh(mesh);
        cache
    }
    // UV/colour updates do not change normals. Compare exact current geometry;
    // Geometry or authored normal changes refresh the stream; generated normals
    // rebuild the weld map only when no authored stream is supplied.
    #[allow(clippy::float_cmp)] // Exact geometry equality is the cache invalidation contract.
    #[doc(hidden)]
    pub fn refresh(&mut self, mesh: &SceneMesh) -> bool {
        let indices_equal = self.indices == mesh.indices;
        self.refresh_with(mesh, indices_equal)
    }
    /// `indices_equal` must be `self.indices == mesh.indices`, computed by the caller.
    pub(super) fn refresh_with(&mut self, mesh: &SceneMesh, indices_equal: bool) -> bool {
        let geometry_changed = !indices_equal
            || self.positions.len() != mesh.vertices.len()
            || !self
                .positions
                .iter()
                .zip(&mesh.vertices)
                .all(|(p, v)| *p == v.position);
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
            self.positions
                .extend(mesh.vertices.iter().map(|v| v.position));
            self.indices.clone_from(&mesh.indices);
        }
        changed
    }
}

// Weld coincident OBJ seam vertices for area-weighted smooth shading. Rebuilt
// after every geometry update, including morphs and displaced liquid layers.
#[doc(hidden)]
pub fn smooth_normals(mesh: &SceneMesh) -> Vec<[f32; 3]> {
    let mut lookup: std::collections::HashMap<(u64, u32), usize, voxy_core::SeededFastHashBuilder> =
        std::collections::HashMap::with_capacity_and_hasher(mesh.vertices.len(), WELD_HASHER);
    let mut mapping = Vec::with_capacity(mesh.vertices.len());
    let mut sums = Vec::<glam::Vec3>::with_capacity(mesh.vertices.len());
    for vertex in &mesh.vertices {
        let key = vertex
            .position
            .map(|x| if x == 0. { 0 } else { x.to_bits() });
        // An injective 96-bit key keeps exact seam identity; the welded result
        // (first-seen index per position) does not depend on the hash function.
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

/// Seeded [`voxy_core::FastHasher`] for the welded 96-bit position key; the
/// randomized `SipHash` default dominated per-frame normal welding on large
/// meshes, and the deterministic fold never affects results, only lookup cost.
const WELD_HASHER: voxy_core::SeededFastHashBuilder =
    voxy_core::SeededFastHashBuilder(0x9E37_79B9_7F4A_7C15);

#[cfg(test)]
mod weld_hasher_tests;
