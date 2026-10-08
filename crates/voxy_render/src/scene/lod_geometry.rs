use super::{NormalCache, SceneError, SceneGeometry, SceneMesh, SceneRenderer, geometry_sizes};
use crate::{
    CertifiedLodIndexSet, LodError, LodIndexSet, LodLevel, LodPolicy, SceneCamera, SceneLodBounds,
};
use std::sync::{Arc, Weak};

/// Selection history owned by one instance/view, without retaining GPU resources.
/// Bundle replacement automatically discards stale previous-level state.
#[derive(Debug, Default)]
pub struct SceneLodHistory {
    bundle: Weak<()>,
    previous: Option<usize>,
}
impl SceneLodHistory {
    /// Clears hysteresis state, for example on a view cut or instance reassignment.
    pub fn reset(&mut self) {
        self.bundle = Weak::new();
        self.previous = None;
    }
    /// Last successful selection, or None when its bundle has been dropped.
    #[must_use]
    pub fn previous(&self) -> Option<usize> {
        self.bundle.upgrade().and(self.previous)
    }
}

/// Immutable GPU geometry variants sharing vertex/material streams. A streaming
/// owner may change optional index-buffer residency through exclusive access.
/// Only shared references escape, so update/clear cannot mutate a shared bundle.
///
/// ```compile_fail
/// fn mutate(bundle: &mut voxy_render::SceneLodGeometry) {
///     bundle.level(0).unwrap().clear();
/// }
/// ```
#[derive(Debug)]
pub struct SceneLodGeometry {
    identity: Arc<()>,
    certified_geometric_errors: bool,
    levels: Vec<Option<SceneGeometry>>,
    streaming_source: Option<Arc<CertifiedLodIndexSet>>,
    metadata: Vec<LodLevel>,
    allocation_bytes: u64,
}
impl SceneLodGeometry {
    /// True only when uploaded from a verified artifact with matching positions.
    /// Covers object-space geometry, not material or projected-camera arithmetic.
    #[must_use]
    pub fn has_certified_geometric_errors(&self) -> bool {
        self.certified_geometric_errors
    }
    /// Selects and publishes history only on success. A different bundle starts
    /// without hysteresis; history does not keep old geometry resident.
    /// # Errors
    /// Propagates camera, bounds and policy validation errors without changing history.
    pub fn select_with_history(
        &self,
        camera: SceneCamera,
        bounds: SceneLodBounds,
        viewport: [u32; 2],
        policy: LodPolicy,
        history: &mut SceneLodHistory,
    ) -> Result<(usize, &SceneGeometry), LodError> {
        let identity = Arc::downgrade(&self.identity);
        let previous = if Weak::ptr_eq(&history.bundle, &identity) {
            history.previous
        } else {
            None
        };
        let selected = self.select_for_camera(camera, bounds, viewport, policy, previous)?;
        history.bundle = identity;
        history.previous = Some(selected.0);
        Ok(selected)
    }

    /// Selects geometry from camera/domain bounds and the actual viewport.
    /// A domain touching/crossing the perspective near plane selects base geometry.
    /// Bounds must enclose all variants; previous-level state belongs to this
    /// instance/view and must be reset when the bundle is replaced.
    /// # Errors
    /// Rejects invalid bounds, cameras, policies and previous-level state,
    /// including when near-plane fallback is required.
    pub fn select_for_camera(
        &self,
        camera: SceneCamera,
        bounds: SceneLodBounds,
        viewport: [u32; 2],
        policy: LodPolicy,
        previous: Option<usize>,
    ) -> Result<(usize, &SceneGeometry), LodError> {
        let desired = self.desired_level_for_camera(camera, bounds, viewport, policy, previous)?;
        self.resident_level(desired)
    }

    /// Selects a borrowed GPU level using its own validated metadata. The caller
    /// owns previous-level state per instance/view and conservative camera bounds.
    /// Near-plane crossing callers use level(0) instead of an unbounded projection.
    /// # Errors
    /// Propagates policy/projection/previous-level validation errors.
    pub fn select(
        &self,
        policy: LodPolicy,
        world_scale: f64,
        pixels_per_world_unit: f64,
        previous: Option<usize>,
    ) -> Result<(usize, &SceneGeometry), LodError> {
        let index = policy.select(&self.metadata, world_scale, pixels_per_world_unit, previous)?;
        self.resident_level(index)
    }
    fn resident_level(&self, index: usize) -> Result<(usize, &SceneGeometry), LodError> {
        // A missing level falls back to the closest resident finer level, which
        // retains the certified error bound. Base geometry is never evicted.
        let (index, geometry) = (0..=index)
            .rev()
            .find_map(|index| self.level(index).map(|geometry| (index, geometry)))
            .ok_or(LodError::InvalidLevels)?;
        Ok((index, geometry))
    }
    #[must_use]
    pub fn level(&self, index: usize) -> Option<&SceneGeometry> {
        self.levels.get(index).and_then(Option::as_ref)
    }
    /// Includes both resident and currently absent certified variants.
    #[must_use]
    pub fn level_count(&self) -> usize {
        self.metadata.len()
    }
    /// Immutable quality/count metadata, including nonresident levels.
    #[must_use]
    pub fn level_metadata(&self, index: usize) -> Option<LodLevel> {
        self.metadata.get(index).copied()
    }
    /// Desired quality independently of residency, for an owner's upload plan.
    /// # Errors
    /// Rejects invalid camera/domain/policy or previous level.
    pub fn desired_level_for_camera(
        &self,
        camera: SceneCamera,
        bounds: SceneLodBounds,
        viewport: [u32; 2],
        policy: LodPolicy,
        previous: Option<usize>,
    ) -> Result<usize, LodError> {
        policy.select_for_camera(&self.metadata, camera, bounds, viewport, previous)
    }
    /// Releases an optional index buffer; shared streams and base stay alive.
    /// The owner must evict only after collecting requirements for every view.
    /// # Errors
    /// Rejects base/out-of-range eviction without changing resident geometry.
    pub fn evict_level(&mut self, index: usize) -> Result<bool, SceneError> {
        if self.streaming_source.is_none() || index == 0 || index >= self.levels.len() {
            return Err(SceneError::InvalidGeometry);
        }
        let Some(geometry) = self.levels[index].take() else {
            return Ok(false);
        };
        self.allocation_bytes -= geometry.indices.size();
        Ok(true)
    }
    /// Actual resident buffer bytes, counting shared streams once and all indices.
    #[must_use]
    pub fn allocation_bytes(&self) -> u64 {
        self.allocation_bytes
    }
}
impl SceneRenderer {
    /// Starts a certified bundle with base geometry only. The immutable CPU
    /// certificate is shared with the importer rather than copied.
    /// # Errors
    /// Rejects certificate/source/device validation failures.
    pub fn upload_streaming_certified_lod_mesh(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
        source: Arc<CertifiedLodIndexSet>,
    ) -> Result<SceneLodGeometry, SceneError> {
        Self::certified_lod_mesh_allocation_bytes(mesh, &source)?;
        let base = self.upload_mesh(device, mesh)?;
        let allocation_bytes = base.allocation_bytes();
        let metadata = source.indices().levels().to_vec();
        let mut levels: Vec<_> = (0..metadata.len()).map(|_| None).collect();
        levels[0] = Some(base);
        Ok(SceneLodGeometry {
            identity: Arc::new(()),
            certified_geometric_errors: true,
            levels,
            streaming_source: Some(source),
            metadata,
            allocation_bytes,
        })
    }
    /// Loads one index buffer after checking total bundle residency. Rejection
    /// preserves all current buffers. Global owners must subtract other resources
    /// from their budget before passing the available bundle budget here.
    /// # Errors
    /// Rejects foreign devices, invalid levels, nonstreaming bundles or budgets.
    pub fn ensure_lod_level(
        &self,
        device: &wgpu::Device,
        target: &mut SceneLodGeometry,
        index: usize,
        bundle_budget: u64,
    ) -> Result<bool, SceneError> {
        let device = self.resource_device(device)?;
        let slot = target
            .levels
            .get(index)
            .ok_or(SceneError::InvalidGeometry)?;
        let base = target.level(0).ok_or(SceneError::InvalidGeometry)?;
        if !base.belongs_to(device) {
            return Err(SceneError::DeviceMismatch);
        }
        if slot.is_some() {
            return Ok(false);
        }
        let source = target
            .streaming_source
            .as_ref()
            .ok_or(SceneError::InvalidGeometry)?;
        let indices = source
            .indices()
            .indices(index)
            .ok_or(SceneError::InvalidGeometry)?;
        geometry_sizes(device, base.vertex_capacity, indices.len())?;
        let bytes = (indices.len() as u64)
            .checked_mul(4)
            .ok_or(SceneError::GeometryCapacityExceeded)?;
        if bytes > bundle_budget.saturating_sub(target.allocation_bytes) {
            return Err(SceneError::GeometryCapacityExceeded);
        }
        let buffer = super::managed_scene_indices(device, indices)?;
        let geometry = SceneGeometry {
            device: device.clone(),
            vertices: base.vertices.clone(),
            normals: base.normals.clone(),
            normal_cache: NormalCache::default(),
            material_coordinates: base.material_coordinates.clone(),
            coordinate_cache: Vec::new(),
            material_parameters: base.material_parameters.clone(),
            index_count: indices.len() as u32,
            vertex_capacity: base.vertex_capacity,
            index_capacity: indices.len(),
            depth_mode: base.depth_mode,
                opaque_shader: base.opaque_shader.clone(),
                partitioned_indices: false,
                partition_cache: Vec::new(),
            indices: buffer,
            shadow_dirty: true,
        };
        target.allocation_bytes += geometry.indices.size();
        target.levels[index] = Some(geometry);
        Ok(true)
    }
    /// Validates exact certified source identity before estimating bundle bytes.
    /// Uses the same unique-stream/all-index accounting as ordinary LOD admission.
    /// # Errors
    /// Rejects changed positions, invalid topology or byte-count overflow.
    pub fn certified_lod_mesh_allocation_bytes(
        mesh: &SceneMesh,
        variants: &CertifiedLodIndexSet,
    ) -> Result<u64, SceneError> {
        if !variants.matches_positions(mesh.vertices.iter().map(|vertex| vertex.position)) {
            return Err(SceneError::InvalidGeometry);
        }
        Self::lod_mesh_allocation_bytes(mesh, variants.indices())
    }

    /// Verifies the mesh still has the exact certified position bits before GPU
    /// creation. Ordinary upload retains its explicitly unverified error contract.
    /// # Errors
    /// Rejects changed positions or propagates source/device/index validation errors.
    pub fn upload_certified_lod_mesh(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
        variants: &CertifiedLodIndexSet,
    ) -> Result<SceneLodGeometry, SceneError> {
        Self::certified_lod_mesh_allocation_bytes(mesh, variants)?;
        let mut geometry = self.upload_lod_mesh(device, mesh, variants.indices())?;
        geometry.certified_geometric_errors = true;
        Ok(geometry)
    }

    /// Stages a verified replacement and preserves the old bundle on rejection.
    /// Caller admission must account for the old-plus-new allocation peak.
    /// # Errors
    /// Propagates certificate/source/device validation without publishing a replacement.
    pub fn replace_certified_lod_mesh(
        &self,
        device: &wgpu::Device,
        target: &mut SceneLodGeometry,
        mesh: &SceneMesh,
        variants: &CertifiedLodIndexSet,
    ) -> Result<(), SceneError> {
        let replacement = self.upload_certified_lod_mesh(device, mesh, variants)?;
        *target = replacement;
        Ok(())
    }

    /// Publishes a new immutable bundle only after successful upload. Validation
    /// failure preserves all previous levels and their allocation accounting.
    /// The caller admits old-plus-new peak bytes before requesting replacement.
    /// # Errors
    /// Propagates upload validation errors without changing the target.
    pub fn replace_lod_mesh(
        &self,
        device: &wgpu::Device,
        target: &mut SceneLodGeometry,
        mesh: &SceneMesh,
        variants: &LodIndexSet,
    ) -> Result<(), SceneError> {
        let replacement = self.upload_lod_mesh(device, mesh, variants)?;
        *target = replacement;
        Ok(())
    }

    /// Logical bytes for one immutable vertex/material stream set and all LOD
    /// index variants. Validates source/domain agreement before admission.
    /// Excludes driver and in-flight overhead; GPU `allocation_bytes` is authoritative.
    /// # Errors
    /// Rejects invalid source/domain/base topology or byte-count overflow.
    pub fn lod_mesh_allocation_bytes(
        mesh: &SceneMesh,
        variants: &LodIndexSet,
    ) -> Result<u64, SceneError> {
        validate_source(mesh, variants)?;
        Self::mesh_allocation_bytes(mesh)
            .checked_sub(mesh.indices.len() as u64 * 4)
            .and_then(|streams| streams.checked_add(variants.index_bytes()))
            .ok_or(SceneError::GeometryCapacityExceeded)
    }

    /// Uploads immutable LOD variants after validating every buffer size and the
    /// source vertex domain/base topology. Levels use original vertex normals and
    /// material coordinates. Existing `SceneDraw` accepts the borrowed selected level.
    /// # Errors
    /// Rejects foreign devices, invalid source geometry or mismatched LOD domains.
    pub fn upload_lod_mesh(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
        variants: &LodIndexSet,
    ) -> Result<SceneLodGeometry, SceneError> {
        let device = self.resource_device(device)?;
        Self::lod_mesh_allocation_bytes(mesh, variants)?;
        for level in variants.levels() {
            geometry_sizes(device, mesh.vertices.len(), level.index_count as usize)?;
        }
        let index_variants: Vec<_> = (0..variants.levels().len())
            .map(|i| variants.indices(i).unwrap_or_default())
            .collect();
        let uploaded = self.upload_mesh_index_variants(
            device,
            mesh,
            &index_variants,
            wgpu::BufferUsages::empty(),
        )?;
        let allocation_bytes = Self::lod_mesh_allocation_bytes(mesh, variants)?;
        let levels = uploaded.into_iter().map(Some).collect();
        Ok(SceneLodGeometry {
            identity: Arc::new(()),
            certified_geometric_errors: false,
            levels,
            streaming_source: None,
            metadata: variants.levels().to_vec(),
            allocation_bytes,
        })
    }
}

fn validate_source(mesh: &SceneMesh, variants: &LodIndexSet) -> Result<(), SceneError> {
    mesh.validate_for_upload()?;
    if variants.vertex_count() != mesh.vertices.len()
        || variants.indices(0) != Some(mesh.indices.as_slice())
    {
        return Err(SceneError::InvalidGeometry);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn streamed_indices_obey_budget_share_streams_and_recover_after_eviction() {
        let (device, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let positions = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let mesh = SceneMesh::new(
            positions
                .map(|position| super::super::SceneVertex {
                    position,
                    uv: [0.; 2],
                    color: [1.; 4],
                })
                .to_vec(),
            vec![0, 1, 2],
        )
        .unwrap();
        let d = crate::LOD_BARYCENTRIC_DENOMINATOR;
        let identity = crate::LodTriangleWitness {
            target_triangle: 0,
            weights: [[d, 0, 0], [0, d, 0], [0, 0, d]],
        };
        let source = Arc::new(
            CertifiedLodIndexSet::new(
                positions.to_vec(),
                vec![0, 1, 2],
                vec![crate::CertifiedLodVariant {
                    indices: vec![0, 1, 2],
                    source_to_variant: vec![identity],
                    variant_to_source: vec![identity],
                }],
            )
            .unwrap(),
        );
        let mut bundle = renderer
            .upload_streaming_certified_lod_mesh(&device, &mesh, source.clone())
            .unwrap();
        let base_bytes = bundle.allocation_bytes();
        assert_eq!(base_bytes, SceneRenderer::mesh_allocation_bytes(&mesh));
        assert!(Arc::ptr_eq(
            bundle.streaming_source.as_ref().unwrap(),
            &source
        ));
        let policy = LodPolicy {
            target_pixels: 1.,
            hysteresis: 0.15,
        };
        assert_eq!(bundle.select(policy, 1., 1., None).unwrap().0, 0);
        assert!(
            renderer
                .ensure_lod_level(&device, &mut bundle, 1, base_bytes + 11)
                .is_err()
        );
        assert_eq!(bundle.allocation_bytes(), base_bytes);
        assert!(bundle.level(1).is_none());
        assert!(
            renderer
                .ensure_lod_level(&device, &mut bundle, 1, base_bytes + 12)
                .unwrap()
        );
        assert_eq!(bundle.allocation_bytes(), base_bytes + 12);
        assert!(std::sync::Arc::ptr_eq(
            &bundle.level(0).unwrap().vertices,
            &bundle.level(1).unwrap().vertices
        ));
        assert!(std::sync::Arc::ptr_eq(
            &bundle.level(0).unwrap().normals,
            &bundle.level(1).unwrap().normals
        ));
        assert_eq!(bundle.select(policy, 1., 1., None).unwrap().0, 1);
        assert!(
            !renderer
                .ensure_lod_level(&device, &mut bundle, 1, base_bytes + 12)
                .unwrap()
        );
        assert!(bundle.evict_level(0).is_err());
        assert!(bundle.evict_level(2).is_err());
        assert!(bundle.evict_level(1).unwrap());
        assert!(!bundle.evict_level(1).unwrap());
        assert_eq!(bundle.allocation_bytes(), base_bytes);
        assert_eq!(bundle.select(policy, 1., 1., Some(1)).unwrap().0, 0);
        assert!(
            renderer
                .ensure_lod_level(&device, &mut bundle, 1, base_bytes + 12)
                .unwrap()
        );
        let (foreign, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        assert!(matches!(
            renderer.ensure_lod_level(&foreign, &mut bundle, 1, base_bytes + 12),
            Err(SceneError::DeviceMismatch)
        ));
        assert_eq!(bundle.allocation_bytes(), base_bytes + 12);

        // The independent device ledger also includes compute and retired LOD
        // indices, beyond the bundle's caller-provided residency limit.
        let (bounded_device, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let shared =
            crate::ComputeMemoryBudget::configure(&bounded_device, base_bytes + 12).unwrap();
        let bounded_renderer = SceneRenderer::new(&bounded_device, wgpu::TextureFormat::Rgba8Unorm);
        let mut bounded = bounded_renderer
            .upload_streaming_certified_lod_mesh(&bounded_device, &mesh, source)
            .unwrap();
        let compute = shared
            .allocate_storage("competing compute", &[0; 12])
            .unwrap();
        let before = shared.stats();
        assert_eq!(
            bounded_renderer.ensure_lod_level(&bounded_device, &mut bounded, 1, base_bytes + 12),
            Err(SceneError::MemoryBudget)
        );
        assert_eq!(shared.stats(), before);
        assert!(bounded.level(1).is_none());
        assert_eq!(bounded.allocation_bytes(), base_bytes);
        drop(compute);
        assert_eq!(
            bounded_renderer.ensure_lod_level(&bounded_device, &mut bounded, 1, base_bytes + 12),
            Err(SceneError::MemoryBudget)
        );
        shared.discard_retired().unwrap();
        assert!(
            bounded_renderer
                .ensure_lod_level(&bounded_device, &mut bounded, 1, base_bytes + 12)
                .unwrap()
        );
        assert_eq!(shared.stats().allocated_bytes, base_bytes + 12);
        assert!(bounded.evict_level(1).unwrap());
        assert_eq!(bounded.allocation_bytes(), base_bytes);
        assert_eq!(shared.stats().retired_buffers, 1);
        assert_eq!(
            bounded_renderer.ensure_lod_level(&bounded_device, &mut bounded, 1, base_bytes + 12),
            Err(SceneError::MemoryBudget)
        );
        shared.discard_retired().unwrap();
        assert!(
            bounded_renderer
                .ensure_lod_level(&bounded_device, &mut bounded, 1, base_bytes + 12)
                .unwrap()
        );
        drop(bounded);
        shared.discard_retired().unwrap();
        assert_eq!(shared.stats().allocated_bytes, 0);
    }
    #[test]
    fn certified_upload_rejects_changed_positions_and_preserves_last_good() {
        let (device, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let positions = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let vertices = positions.map(|position| super::super::SceneVertex {
            position,
            uv: [0.; 2],
            color: [1.; 4],
        });
        let mut mesh = SceneMesh::new(vertices.to_vec(), vec![0, 1, 2]).unwrap();
        let certificate =
            CertifiedLodIndexSet::new(positions.to_vec(), mesh.indices.clone(), vec![]).unwrap();
        let plain = renderer
            .upload_lod_mesh(&device, &mesh, certificate.indices())
            .unwrap();
        assert!(!plain.has_certified_geometric_errors());
        let mut published = renderer
            .upload_certified_lod_mesh(&device, &mesh, &certificate)
            .unwrap();
        assert!(published.has_certified_geometric_errors());
        let bytes = published.allocation_bytes();
        assert_eq!(
            SceneRenderer::certified_lod_mesh_allocation_bytes(&mesh, &certificate).unwrap(),
            bytes
        );
        let buffer = published.level(0).unwrap().vertices.clone();
        mesh.vertices[0].position[2] = 1.0;
        assert!(SceneRenderer::certified_lod_mesh_allocation_bytes(&mesh, &certificate).is_err());
        assert!(matches!(
            renderer.replace_certified_lod_mesh(&device, &mut published, &mesh, &certificate),
            Err(SceneError::InvalidGeometry)
        ));
        assert!(published.has_certified_geometric_errors());
        assert_eq!(published.allocation_bytes(), bytes);
        assert!(std::sync::Arc::ptr_eq(
            &published.level(0).unwrap().vertices,
            &buffer
        ));
        mesh.vertices[0].position[2] = 0.0;
        mesh.indices = vec![0, 2, 1];
        assert!(matches!(
            renderer.upload_certified_lod_mesh(&device, &mesh, &certificate),
            Err(SceneError::InvalidGeometry)
        ));
    }

    #[test]
    fn lod_history_replacement_failure_and_resource_lifetime() {
        let (device, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let vertices = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]].map(|position| {
            super::super::SceneVertex {
                position,
                uv: [0.0; 2],
                color: [1.0; 4],
            }
        });
        let mesh = SceneMesh::new(vertices.to_vec(), vec![0, 1, 2]).unwrap();
        let variants =
            LodIndexSet::new(3, vec![(0.0, vec![0, 1, 2]), (0.01, vec![0, 1, 2])]).unwrap();
        let bundle = renderer.upload_lod_mesh(&device, &mesh, &variants).unwrap();
        let camera = SceneCamera {
            eye: glam::Vec3::ZERO,
            target: glam::Vec3::NEG_Z,
            up: glam::Vec3::Y,
            projection: crate::SceneProjection::Perspective {
                vertical_fov: 1.0,
                aspect: 1.0,
                near: 0.1,
                far: 100.0,
            },
        };
        let bounds = camera
            .lod_bounds(
                glam::Mat4::from_translation(glam::Vec3::new(0.0, 0.0, -10.0)),
                glam::Vec3::ZERO,
                glam::Vec3::ONE,
            )
            .unwrap();
        let policy = LodPolicy {
            target_pixels: 1.0,
            hysteresis: 0.1,
        };
        let mut history = SceneLodHistory::default();
        assert_eq!(history.previous(), None);
        assert_eq!(
            bundle
                .select_with_history(camera, bounds, [800, 800], policy, &mut history)
                .unwrap()
                .0,
            1
        );
        assert_eq!(history.previous(), Some(1));
        verify_independent_hysteresis(&bundle, camera, bounds, &mut history);
        let invalid = SceneLodBounds {
            error_scale: f64::NAN,
            ..bounds
        };
        assert!(
            bundle
                .select_with_history(camera, invalid, [800, 800], policy, &mut history)
                .is_err()
        );
        assert_eq!(history.previous(), Some(1));
        let base = LodIndexSet::new(3, vec![(0.0, vec![0, 1, 2])]).unwrap();
        let replacement = renderer.upload_lod_mesh(&device, &mesh, &base).unwrap();
        assert_eq!(
            replacement
                .select_with_history(camera, bounds, [800, 800], policy, &mut history)
                .unwrap()
                .0,
            0
        );
        assert_eq!(history.previous(), Some(0));
        drop(replacement);
        assert_eq!(history.previous(), None);
        bundle
            .select_with_history(camera, bounds, [800, 800], policy, &mut history)
            .unwrap();
        history.reset();
        assert_eq!(history.previous(), None);
    }

    fn verify_independent_hysteresis(
        bundle: &SceneLodGeometry,
        camera: SceneCamera,
        bounds: SceneLodBounds,
        history: &mut SceneLodHistory,
    ) {
        let policy = LodPolicy {
            target_pixels: 0.8,
            hysteresis: 0.1,
        };
        assert_eq!(
            bundle
                .select_with_history(camera, bounds, [800, 800], policy, history)
                .unwrap()
                .0,
            1
        );
        let mut independent = SceneLodHistory::default();
        assert_eq!(
            bundle
                .select_with_history(camera, bounds, [800, 800], policy, &mut independent)
                .unwrap()
                .0,
            0
        );
        assert_eq!(history.previous(), Some(1));
        assert_eq!(independent.previous(), Some(0));
    }

    #[test]
    fn lod_camera_selection_falls_back_and_validates_history() {
        let (device, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let vertices = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]].map(|position| {
            super::super::SceneVertex {
                position,
                uv: [0.0; 2],
                color: [1.0; 4],
            }
        });
        let mesh = SceneMesh::new(vertices.to_vec(), vec![0, 1, 2]).unwrap();
        let variants =
            LodIndexSet::new(3, vec![(0.0, vec![0, 1, 2]), (0.01, vec![0, 1, 2])]).unwrap();
        let bundle = renderer.upload_lod_mesh(&device, &mesh, &variants).unwrap();
        let camera = SceneCamera {
            eye: glam::Vec3::ZERO,
            target: glam::Vec3::NEG_Z,
            up: glam::Vec3::Y,
            projection: crate::SceneProjection::Perspective {
                vertical_fov: 1.0,
                aspect: 1.0,
                near: 0.1,
                far: 100.0,
            },
        };
        let policy = LodPolicy {
            target_pixels: 1.0,
            hysteresis: 0.1,
        };
        let far = camera
            .lod_bounds(
                glam::Mat4::from_translation(glam::Vec3::new(0.0, 0.0, -10.0)),
                glam::Vec3::ZERO,
                glam::Vec3::ONE,
            )
            .unwrap();
        let select = |bounds, previous| {
            bundle.select_for_camera(camera, bounds, [800, 800], policy, previous)
        };
        assert_eq!(select(far, None).unwrap().0, 1);
        let near = camera
            .lod_bounds(glam::Mat4::IDENTITY, -glam::Vec3::ONE, glam::Vec3::ONE)
            .unwrap();
        assert_eq!(select(near, Some(1)).unwrap().0, 0);
        assert!(matches!(
            select(near, Some(2)),
            Err(LodError::InvalidPreviousLevel)
        ));
        let invalid = SceneLodBounds {
            error_scale: f64::NAN,
            ..near
        };
        assert!(matches!(
            select(invalid, None),
            Err(LodError::InvalidProjection)
        ));
    }

    #[test]
    fn lod_upload_rejects_foreign_device_and_vertex_domain_before_allocation() {
        let (device, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let (foreign, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let vertex = |position| super::super::SceneVertex {
            position,
            uv: [0.0; 2],
            color: [1.0; 4],
        };
        let mesh = SceneMesh::new(
            vec![
                vertex([0.0, 0.0, 0.0]),
                vertex([1.0, 0.0, 0.0]),
                vertex([0.0, 1.0, 0.0]),
            ],
            vec![0, 1, 2],
        )
        .unwrap();
        let valid = LodIndexSet::new(3, vec![(0.0, vec![0, 1, 2])]).unwrap();
        assert_eq!(
            SceneRenderer::lod_mesh_allocation_bytes(&mesh, &valid).unwrap(),
            SceneRenderer::mesh_allocation_bytes(&mesh)
        );
        assert!(matches!(
            renderer.upload_lod_mesh(&foreign, &mesh, &valid),
            Err(SceneError::DeviceMismatch)
        ));
        let wrong_domain = LodIndexSet::new(4, vec![(0.0, vec![0, 1, 2])]).unwrap();
        let mut published = renderer.upload_lod_mesh(&device, &mesh, &valid).unwrap();
        let previous_buffer = published.level(0).unwrap().vertices.clone();
        let previous_bytes = published.allocation_bytes();
        assert!(
            renderer
                .replace_lod_mesh(&device, &mut published, &mesh, &wrong_domain)
                .is_err()
        );
        assert!(std::sync::Arc::ptr_eq(
            &published.level(0).unwrap().vertices,
            &previous_buffer
        ));
        assert_eq!(published.allocation_bytes(), previous_bytes);
        assert!(matches!(
            renderer.upload_lod_mesh(&device, &mesh, &wrong_domain),
            Err(SceneError::InvalidGeometry)
        ));
    }

    #[test]
    #[ignore = "requires a real GPU adapter; run explicitly for LOD acceptance"]
    fn gpu_lod_upload_shares_streams_and_counts_all_index_buffers() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .expect("GPU adapter required for LOD acceptance");
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let vertex = |position| super::super::SceneVertex {
            position,
            uv: [0.0; 2],
            color: [1.0; 4],
        };
        let mesh = SceneMesh::new(
            vec![
                vertex([0.0, 0.0, 0.0]),
                vertex([1.0, 0.0, 0.0]),
                vertex([1.0, 1.0, 0.0]),
                vertex([0.0, 1.0, 0.0]),
            ],
            vec![0, 1, 2, 0, 2, 3],
        )
        .unwrap();
        let variants = LodIndexSet::new(
            4,
            vec![(0.0, vec![0, 1, 2, 0, 2, 3]), (0.25, vec![0, 1, 2])],
        )
        .unwrap();
        let bundle = renderer.upload_lod_mesh(&device, &mesh, &variants).unwrap();
        assert_eq!(
            SceneRenderer::lod_mesh_allocation_bytes(&mesh, &variants).unwrap(),
            bundle.allocation_bytes()
        );
        let base = bundle.level(0).unwrap();
        let (chosen, coarse) = bundle
            .select(
                LodPolicy {
                    target_pixels: 1.0,
                    hysteresis: 0.0,
                },
                1.0,
                2.0,
                None,
            )
            .unwrap();
        assert_eq!(chosen, 1);
        assert_eq!(
            bundle
                .select(
                    LodPolicy {
                        target_pixels: 1.0,
                        hysteresis: 0.0
                    },
                    1.0,
                    10.0,
                    None
                )
                .unwrap()
                .0,
            0
        );
        assert!(std::sync::Arc::ptr_eq(&base.vertices, &coarse.vertices));
        assert!(std::sync::Arc::ptr_eq(&base.normals, &coarse.normals));
        assert!(std::sync::Arc::ptr_eq(
            &base.material_coordinates,
            &coarse.material_coordinates
        ));
        assert!(std::sync::Arc::ptr_eq(
            &base.material_parameters,
            &coarse.material_parameters
        ));
        assert!(!std::sync::Arc::ptr_eq(&base.indices, &coarse.indices));
        let fine_pixels = rendered_pixels(&renderer, &device, &queue, base);
        let coarse_pixels = rendered_pixels(&renderer, &device, &queue, coarse);
        assert!(fine_pixels > coarse_pixels && coarse_pixels > 0);

        assert_eq!(base.index_count(), 6);
        assert_eq!(coarse.index_count(), 3);
        assert_eq!(
            bundle.allocation_bytes(),
            base.allocation_bytes() + coarse.indices.size()
        );
        let mismatched = LodIndexSet::new(4, vec![(0.0, vec![0, 1, 2])]).unwrap();
        assert!(
            renderer
                .upload_lod_mesh(&device, &mesh, &mismatched)
                .is_err()
        );
        assert!(pollster::block_on(scope.pop()).is_none());
    }
    fn rendered_pixels(
        renderer: &SceneRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        geometry: &SceneGeometry,
    ) -> usize {
        let make_texture = |format, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("LOD pixel acceptance"),
                size: wgpu::Extent3d {
                    width: 8,
                    height: 8,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let color = make_texture(
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        );
        let depth = make_texture(
            wgpu::TextureFormat::Depth32Float,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        let texture = renderer
            .upload_texture(device, queue, 1, 1, &[255; 4])
            .unwrap();
        let transform = renderer
            .create_transform(device, glam::Mat4::IDENTITY)
            .unwrap();
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LOD pixels"),
            size: 256 * 8,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        renderer.encode(
            &mut encoder,
            &color.create_view(&wgpu::TextureViewDescriptor::default()),
            &depth.create_view(&wgpu::TextureViewDescriptor::default()),
            wgpu::Color::BLACK,
            &[super::super::SceneDraw {
                geometry,
                texture: &texture,
                transform: &transform,
                overlay: false,
            }],
        );
        encoder.copy_texture_to_buffer(
            color.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(8),
                },
            },
            wgpu::Extent3d {
                width: 8,
                height: 8,
                depth_or_array_layers: 1,
            },
        );
        let submitted = queue.submit([encoder.finish()]);
        let (send, receive) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                send.send(result).unwrap();
            });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submitted),
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .unwrap();
        receive
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap()
            .unwrap();
        let mapped = readback.slice(..).get_mapped_range().unwrap();
        (0..8)
            .flat_map(|row| (0..8).map(move |column| row * 256 + column * 4))
            .filter(|offset| mapped[*offset..*offset + 3].iter().any(|value| *value != 0))
            .count()
    }
}
