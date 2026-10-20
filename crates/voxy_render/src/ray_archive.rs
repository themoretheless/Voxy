//! Full source meshes share immutable attribute/index buffers and one BLAS per mesh.
use super::{RayGeometry, RayInstanceUpdate, RayScene, RaySceneError, validate_transform};
use crate::archive_scene::ArchiveMeshSource;

#[derive(Debug)]
pub(super) struct MeshRange {
    pub size: wgpu::BlasAABBGeometrySizeDescriptor,
    pub offset: u32,
    pub source: [u32; 5],
}
#[derive(Debug)]
pub(super) struct RayArchiveGeometry {
    pub bounds: crate::ComputeStorage,
    pub exact_metadata: Option<(Vec<[u32; 4]>, Vec<u32>)>,
    pub blases: Vec<wgpu::Blas>,
    pub ranges: Vec<MeshRange>,
    pub instance_mesh: Vec<usize>,
    pub source_mesh_count: usize,
    pub source_instances: crate::ComputeStorage,
    pub instance_layout: crate::ComputeStorage,
    pub proxy_instances: Vec<bool>,
    pub native_transforms: Vec<[f32; 12]>,
}

const IDENTITY: [f32; 12] = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.];

fn affine_transform(world: [[f32; 4]; 4]) -> Result<[f32; 12], RaySceneError> {
    if world.iter().flatten().any(|v| !v.is_finite())
        || [world[0][3], world[1][3], world[2][3], world[3][3]] != [0., 0., 0., 1.]
    {
        return Err(RaySceneError::InvalidTransform);
    }
    Ok([
        world[0][0],
        world[1][0],
        world[2][0],
        world[3][0],
        world[0][1],
        world[1][1],
        world[2][1],
        world[3][1],
        world[0][2],
        world[1][2],
        world[2][2],
        world[3][2],
    ])
}
#[cfg(test)]
fn source_transform(world: [[f32; 4]; 4]) -> Result<[f32; 12], RaySceneError> {
    let transform = affine_transform(world)?;
    validate_transform(transform)?;
    Ok(transform)
}

// A rejection-only world-space envelope, not replacement vertex positions.
// Directed f64 operations enclose the real affine image of the original f32 inputs.
fn world_bounds(
    points: [[f32; 3]; 3],
    world: [[f32; 4]; 4],
    padding: f32,
) -> Result<[f32; 6], RaySceneError> {
    let mut result = [0.; 6];
    for axis in 0..3 {
        let mut low = f64::INFINITY;
        let mut high = f64::NEG_INFINITY;
        for p in points {
            let mut lo = f64::from(world[3][axis]);
            let mut hi = lo;
            for k in 0..3 {
                let product = f64::from(world[k][axis]) * f64::from(p[k]);
                lo = (lo + product.next_down()).next_down();
                hi = (hi + product.next_up()).next_up();
            }
            low = low.min(lo);
            high = high.max(hi);
        }
        result[axis] = ((low - f64::from(padding)).next_down() as f32).next_down();
        result[axis + 3] = ((high + f64::from(padding)).next_up() as f32).next_up();
    }
    if result.iter().any(|v| !v.is_finite()) {
        return Err(RaySceneError::InvalidGeometry);
    }
    Ok(result)
}

impl RayScene {
    /// Creates procedural triangle candidates for every source mesh and instance.
    /// Full 48-byte vertex attributes and local indices are retained without conversion.
    /// Singular source instances use world-space candidate BLASes with identity TLAS
    /// transforms; source vertex and instance bytes remain unchanged. Such proxy
    /// transforms cannot be edited without rebuilding geometry.
    /// `padding` is a local margin for ordinary BLASes and a world margin for proxies.
    /// It is not a proof of compensated-ray candidate coverage.
    /// Consumers must implement exact triangle intersection and material/alpha filtering.
    /// This constructor never makes procedural candidates opaque committed hits.
    /// # Errors
    /// Rejects unsupported features, empty meshes, invalid source data or exceeded limits.
    pub fn from_archive_candidates(
        device: &wgpu::Device,
        meshes: &[ArchiveMeshSource],
        padding: f32,
    ) -> Result<Self, RaySceneError> {
        Self::from_archive_candidates_impl(device,meshes,padding,false)
    }
    /// Retain the same original source/native geometry with a conservative
    /// hierarchy over groups of at most eight stable source primitives.
    /// Selection belongs to this immutable scene owner, independently of other scenes.
    pub fn from_archive_candidates_with_primitive_groups(
        device:&wgpu::Device,meshes:&[ArchiveMeshSource],padding:f32,
    )->Result<Self,RaySceneError> {
        Self::from_archive_candidates_impl(device,meshes,padding,true)
    }
    fn from_archive_candidates_impl(
        device:&wgpu::Device,meshes:&[ArchiveMeshSource],padding:f32,primitive_groups:bool,
    )->Result<Self,RaySceneError> {
        if !device
            .features()
            .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
        {
            return Err(RaySceneError::Unsupported);
        }
        if meshes.is_empty() || !padding.is_finite() || padding < 0. {
            return Err(RaySceneError::InvalidGeometry);
        }
        let limits = device.limits();
        if limits.max_blas_geometry_count == 0 {
            return Err(RaySceneError::Capacity);
        }
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut boxes = Vec::new();
        let mut ranges = Vec::new();
        let mut updates = Vec::new();
        let mut instance_mesh = Vec::new();
        let mut source_instances = Vec::new();
        let mut instance_layout = Vec::<[u32; 4]>::new();
        let mut proxy_instances = Vec::new();
        let mut proxies = Vec::new();
        for (mesh_id, mesh) in meshes.iter().enumerate() {
            if mesh.indices.is_empty()
                || mesh.indices.len() % 3 != 0
                || mesh
                    .indices
                    .iter()
                    .any(|&i| i as usize >= mesh.vertices.len())
                || bytemuck::cast_slice::<_, f32>(&mesh.vertices)
                    .iter()
                    .any(|v| !v.is_finite())
            {
                return Err(RaySceneError::InvalidGeometry);
            }
            let primitive_count =
                u32::try_from(mesh.indices.len() / 3).map_err(|_| RaySceneError::Capacity)?;
            if primitive_count > limits.max_blas_primitive_count {
                return Err(RaySceneError::Capacity);
            }
            let offset = u32::try_from(boxes.len() * 24).map_err(|_| RaySceneError::Capacity)?;
            ranges.push(MeshRange {
                offset,
                source: [
                    u32::try_from(vertices.len()).map_err(|_| RaySceneError::Capacity)?,
                    u32::try_from(mesh.vertices.len()).map_err(|_| RaySceneError::Capacity)?,
                    u32::try_from(indices.len()).map_err(|_| RaySceneError::Capacity)?,
                    u32::try_from(mesh.indices.len()).map_err(|_| RaySceneError::Capacity)?,
                    u32::try_from(mesh.material).map_err(|_| RaySceneError::Capacity)?,
                ],
                size: wgpu::BlasAABBGeometrySizeDescriptor {
                    primitive_count,
                    flags: wgpu::AccelerationStructureGeometryFlags::empty(),
                },
            });
            for triangle in mesh.indices.chunks_exact(3) {
                let p = [
                    mesh.vertices[triangle[0] as usize].position,
                    mesh.vertices[triangle[1] as usize].position,
                    mesh.vertices[triangle[2] as usize].position,
                ];
                let mut bounds = [0.; 6];
                for axis in 0..3 {
                    bounds[axis] =
                        (p[0][axis].min(p[1][axis]).min(p[2][axis]) - padding).next_down();
                    bounds[axis + 3] =
                        (p[0][axis].max(p[1][axis]).max(p[2][axis]) + padding).next_up();
                }
                if bounds.iter().any(|v| !v.is_finite()) {
                    return Err(RaySceneError::InvalidGeometry);
                }
                boxes.push(bounds);
            }
            for instance in &mesh.instances {
                let index = u32::try_from(updates.len()).map_err(|_| RaySceneError::Capacity)?;
                if index >= limits.max_tlas_instance_count || index > 0x00ff_ffff {
                    return Err(RaySceneError::Capacity);
                }
                let transform = affine_transform(instance.world)?;
                let proxy = validate_transform(transform).is_err();
                if !proxy && instance.normal.iter().flatten().any(|v| !v.is_finite()) {
                    return Err(RaySceneError::InvalidGeometry);
                }
                if proxy {
                    proxies.push((index, mesh_id, instance.world));
                }
                source_instances.push(*instance);
                let source = ranges[mesh_id].source;
                instance_layout.push([mesh_id as u32, source[0], source[2], source[4]]);
                proxy_instances.push(proxy);
                updates.push(RayInstanceUpdate {
                    index,
                    transform: if proxy { IDENTITY } else { transform },
                    custom_index: index,
                    mask: 255,
                });
                instance_mesh.push(mesh_id);
            }
            vertices.extend_from_slice(&mesh.vertices);
            indices.extend_from_slice(&mesh.indices);
        }
        // Singular sources cannot be inverted by a native TLAS. Give each such
        // source its own world-space candidate BLAS and identity native instance.
        // Original vertices, indices, transforms, normals and stable IDs stay intact.
        for (slot, mesh_id, world) in proxies {
            let source = ranges[mesh_id].source;
            let size = ranges[mesh_id].size.clone();
            let offset = u32::try_from(boxes.len() * 24).map_err(|_| RaySceneError::Capacity)?;
            instance_mesh[slot as usize] = ranges.len();
            ranges.push(MeshRange {
                size,
                offset,
                source,
            });
            let mesh = &meshes[mesh_id];
            for triangle in mesh.indices.chunks_exact(3) {
                let points = std::array::from_fn(|k| mesh.vertices[triangle[k] as usize].position);
                boxes.push(world_bounds(points, world, padding)?);
            }
        }
        u32::try_from(boxes.len()).map_err(|_| RaySceneError::Capacity)?;
        if updates.is_empty() {
            return Err(RaySceneError::InvalidGeometry);
        }
        let contents = [
            bytemuck::cast_slice(&vertices),
            bytemuck::cast_slice(&indices),
            bytemuck::cast_slice(&boxes),
            bytemuck::cast_slice(&source_instances),
            bytemuck::cast_slice(&instance_layout),
        ];
        if contents
            .iter()
            .any(|b| b.len() as u64 > limits.max_buffer_size)
        {
            return Err(RaySceneError::Capacity);
        }
        let [vertices, indices, bounds, source_instances, instance_layout] =
            crate::ComputeMemoryBudget::for_device(device)
                .allocate_buffers([
                    voxy_gpu_core::ManagedBufferDescriptor {
                        label: "world ray source vertices",
                        size: contents[0].len() as u64,
                        contents: Some(contents[0]),
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    },
                    voxy_gpu_core::ManagedBufferDescriptor {
                        label: "world ray source indices",
                        size: contents[1].len() as u64,
                        contents: Some(contents[1]),
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    },
                    voxy_gpu_core::ManagedBufferDescriptor {
                        label: "world ray triangle bounds",
                        size: contents[2].len() as u64,
                        contents: Some(contents[2]),
                        usage: wgpu::BufferUsages::BLAS_INPUT,
                    },
                    voxy_gpu_core::ManagedBufferDescriptor {
                        label: "world ray original instances",
                        size: contents[3].len() as u64,
                        contents: Some(contents[3]),
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    },
                    voxy_gpu_core::ManagedBufferDescriptor {
                        label: "world ray stable source layout",
                        size: contents[4].len() as u64,
                        contents: Some(contents[4]),
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    },
                ])
                .map_err(|_| RaySceneError::Capacity)?;
        let blases: Vec<_> = ranges
            .iter()
            .map(|range| {
                device.create_blas(
                    &wgpu::CreateBlasDescriptor {
                        label: Some("world mesh procedural BLAS"),
                        flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
                        update_mode: wgpu::AccelerationStructureUpdateMode::Build,
                    },
                    wgpu::BlasGeometrySizeDescriptors::AABBs {
                        descriptors: vec![range.size.clone()],
                    },
                )
            })
            .collect();
        let exact_metadata = if meshes.len() <= 2048 {
            let mut counts = vec![[0u32; 4]; 4096];
            for (id, mesh) in meshes.iter().enumerate() {
                counts[id * 2][0] = (mesh.indices.len() / 3) as u32;
                for axis in 0..3 {
                    counts[id * 2][axis + 1] = mesh
                        .vertices
                        .iter()
                        .map(|v| v.position[axis])
                        .fold(f32::INFINITY, f32::min)
                        .to_bits();
                    counts[id * 2 + 1][axis] = mesh
                        .vertices
                        .iter()
                        .map(|v| v.position[axis])
                        .fold(f32::NEG_INFINITY, f32::max)
                        .to_bits();
                }
            }
            let masks = vec![255u32; instance_mesh.len()];
            let hierarchy = if primitive_groups {crate::world_nearest_bvh::pack_groups(meshes,&masks)}else{crate::world_nearest_bvh::pack(meshes,&masks)}
                .map_err(|_| RaySceneError::InvalidGeometry)?;
            Some((counts, hierarchy))
        } else {
            None
        };
        let primary = blases[0].clone();
        let capacity = updates.len() as u32;
        let mut scene = Self::finish_create(
            device,
            RayGeometry::Archive(RayArchiveGeometry {
                bounds,
                exact_metadata,
                blases,
                ranges,
                instance_mesh,
                source_mesh_count: meshes.len(),
                source_instances,
                instance_layout,
                proxy_instances,
                native_transforms: updates.iter().map(|u| u.transform).collect(),
            }),
            primary,
            vertices,
            Some(indices),
            capacity,
        );
        scene.set_instances(&updates)?;
        Ok(scene)
    }
}

impl RayScene {
    /// Derived bounds/hierarchy from this immutable archive owner, with current masks.
    /// No caller-supplied mesh can replace the source metadata used for exact queries.
    #[doc(hidden)]
    pub fn archive_exact_source_metadata(&self) -> Option<(Vec<[u32; 4]>, Vec<u32>)> {
        let RayGeometry::Archive(archive) = &self.geometry else {
            return None;
        };
        let (counts, hierarchy) = archive.exact_metadata.as_ref()?;
        let mut hierarchy = hierarchy.clone();
        for (word, instance) in hierarchy[4..4 + self.instances.len()]
            .iter_mut()
            .zip(&self.instances)
        {
            *word = instance.as_ref().map_or(0, |v| u32::from(v.mask));
        }
        Some((counts.clone(), hierarchy))
    }
    /// Snapshot of active source masks. Use with the same built TLAS revision.
    pub fn archive_candidate_active_masks(&self) -> Option<Vec<u32>> {
        match &self.geometry {
            RayGeometry::Archive(_) => Some(
                self.instances
                    .iter()
                    .map(|v| v.as_ref().map_or(0, |v| u32::from(v.mask)))
                    .collect(),
            ),
            _ => None,
        }
    }
    /// Immutable source layout: first vertex, vertex count, first index, index count, material.
    /// Local indices remain local to their mesh; no instance geometry is flattened.
    pub fn archive_candidate_layout(&self) -> Option<Vec<[u32; 5]>> {
        match &self.geometry {
            RayGeometry::Archive(archive) => Some(
                archive.ranges[..archive.source_mesh_count]
                    .iter()
                    .map(|r| r.source)
                    .collect(),
            ),
            _ => None,
        }
    }
}

impl RayScene {
    pub fn archive_candidate_proxy_count(&self) -> Option<usize> {
        match &self.geometry {
            RayGeometry::Archive(a) => Some(a.proxy_instances.iter().filter(|&&p| p).count()),
            _ => None,
        }
    }
    /// Original vertex, local index, and instance buffers, in that order.
    /// Raw source bytes include singular matrices and their original normal data.
    /// Proxy narrow phase must use these source matrices instead of native transforms.
    pub fn archive_candidate_source_bindings(&self) -> Option<[wgpu::BindingResource<'_>; 3]> {
        match &self.geometry {
            RayGeometry::Archive(a) => Some([
                self.vertices.as_entire_binding(),
                self.indices.as_ref()?.as_entire_binding(),
                a.source_instances.as_entire_binding(),
            ]),
            _ => None,
        }
    }
    /// Per stable instance: source mesh ID, first vertex, first local index, material ID.
    /// Source mesh identity is preserved when a singular instance uses a proxy BLAS.
    pub fn archive_candidate_instance_layout(&self) -> Option<wgpu::BindingResource<'_>> {
        match &self.geometry {
            RayGeometry::Archive(a) => Some(a.instance_layout.as_entire_binding()),
            _ => None,
        }
    }
    pub(super) fn validate_archive_native_transform(
        &self,
        index: u32,
        transform: [f32; 12],
        custom_index: u32,
    ) -> Result<(), RaySceneError> {
        if let RayGeometry::Archive(a) = &self.geometry {
            // Source instance metadata is immutable until a new archive is built.
            // Reject updates that would silently desynchronize it from native TLAS.
            if a.native_transforms[index as usize] != transform || custom_index != index {
                return Err(RaySceneError::InvalidTransform);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn singular_world_bounds_keep_flat_geometry_and_enclose_cancellation() {
        let points = [[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]];
        let world = [
            [0.28, 0., 0., 0.],
            [0., 2.3, 0., 0.],
            [0., 0., 0., 0.],
            [9.5, 1.15, 14.5, 1.],
        ];
        let bounds = world_bounds(points, world, 0.).unwrap();
        assert!(bounds[2] < 14.5 && bounds[5] > 14.5);
        for point in points {
            for axis in 0..3 {
                let value = f64::from(world[3][axis])
                    + (0..3)
                        .map(|k| f64::from(world[k][axis]) * f64::from(point[k]))
                        .sum::<f64>();
                assert!(f64::from(bounds[axis]) <= value && value <= f64::from(bounds[axis + 3]));
            }
        }
        let cancel = [
            [1e20, 0., 0., 0.],
            [-1e20, 1., 0., 0.],
            [1., 0., 0., 0.],
            [1., 0., 0., 1.],
        ];
        let points = [[1., 1., 1.]; 3];
        let bounds = world_bounds(points, cancel, 0.).unwrap();
        assert!(bounds[0] <= 2. && bounds[3] >= 2.);
    }
    #[test]
    fn source_matrix_layout_keeps_translation_and_signed_scales() {
        let world = [
            [-2., 0., 0., 0.],
            [0., 3., 0., 0.],
            [0., 0., 4., 0.],
            [5., 6., 7., 1.],
        ];
        assert_eq!(
            source_transform(world).unwrap(),
            [-2., 0., 0., 5., 0., 3., 0., 6., 0., 0., 4., 7.]
        );
    }
    #[test]
    fn singular_and_projective_source_instances_are_explicit_errors() {
        let mut world = glam::Mat4::IDENTITY.to_cols_array_2d();
        world[2][2] = 0.;
        assert_eq!(
            source_transform(world),
            Err(RaySceneError::InvalidTransform)
        );
        world[2][2] = 1.;
        world[0][3] = 0.25;
        assert_eq!(
            source_transform(world),
            Err(RaySceneError::InvalidTransform)
        );
    }
}
