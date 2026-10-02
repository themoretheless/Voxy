//! Validated triangle acceleration structures for devices with ray queries enabled.
use wgpu::util::DeviceExt;

/// Opaque segment visibility compute shader: bindings TLAS, segments, u32 outputs.
/// Output is one for unobstructed, zero for blocked; use only validated segments.
pub const RAY_VISIBILITY_SHADER: &str = include_str!("ray_visibility.wgsl");

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RaySegment {
    origin_bias: [f32; 4],
    target: [f32; 4],
}
impl RaySegment {
    /// Creates a finite shadow segment with positive endpoint bias in world units.
    /// # Errors
    /// Rejects invalid/overflowing endpoints, bias or a segment shorter than twice bias.
    pub fn new(origin: [f32; 3], target: [f32; 3], bias: f32) -> Result<Self, RaySceneError> {
        let distance = (glam::Vec3::from_array(target) - glam::Vec3::from_array(origin)).length();
        if origin.iter().chain(target.iter()).any(|v| !v.is_finite())
            || !bias.is_finite()
            || bias <= 0.0
            || !distance.is_finite()
            || distance <= 2.0 * bias
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        Ok(Self {
            origin_bias: [origin[0], origin[1], origin[2], bias],
            target: [target[0], target[1], target[2], 0.0],
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RaySceneError {
    DeviceMismatch,
    Unsupported,
    InvalidGeometry,
    InvalidTransform,
    Capacity,
}
impl std::fmt::Display for RaySceneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ray scene error: {self:?}")
    }
}
impl std::error::Error for RaySceneError {}

#[derive(Debug)]
pub struct RayScene {
    device: wgpu::Device,
    geometry_revision: u64,
    instances: Vec<Option<RayInstanceUpdate>>,
    vertices: wgpu::Buffer,
    indices: Option<wgpu::Buffer>,
    geometry: wgpu::BlasTriangleGeometrySizeDescriptor,
    blas: wgpu::Blas,
    tlas: wgpu::Tlas,
    instance_capacity: u32,
}
/// One shared-BLAS instance update in a transactionally validated batch.
#[derive(Clone, Copy, Debug)]
pub struct RayInstanceUpdate {
    pub index: u32,
    pub transform: [f32; 12],
    pub custom_index: u32,
    pub mask: u8,
}

impl RayScene {
    pub(crate) fn validate_device(&self, device: &wgpu::Device) -> Result<(), RaySceneError> {
        if &self.device != device {
            return Err(RaySceneError::DeviceMismatch);
        }
        Ok(())
    }

    /// Updates the instance's row-major 3x4 affine object-to-world transform.
    /// Call `build_instances` after the initial full `build` and before querying.
    /// Submit previous queries before changing the instance; this is not a GPU fence.
    /// # Errors
    /// Rejects non-finite or singular transforms before changing the TLAS instance.
    pub fn set_transform(&mut self, transform: [f32; 12]) -> Result<(), RaySceneError> {
        self.set_instance(0, transform, 0, 255)
    }

    /// Sets one shared-BLAS instance; custom indices are limited to 24 bits.
    /// # Errors
    /// Rejects capacity/custom-index overflow or invalid transforms atomically.
    pub fn set_instance(
        &mut self,
        index: u32,
        transform: [f32; 12],
        custom_index: u32,
        mask: u8,
    ) -> Result<(), RaySceneError> {
        if index >= self.instance_capacity || custom_index > 0x00ff_ffff {
            return Err(RaySceneError::Capacity);
        }
        validate_transform(transform)?;
        self.instances[index as usize] = Some(RayInstanceUpdate {
            index,
            transform,
            custom_index,
            mask,
        });
        self.tlas[index as usize] = Some(wgpu::TlasInstance::new(
            &self.blas,
            transform,
            custom_index,
            mask,
        ));
        Ok(())
    }

    /// Validate every update before changing any TLAS instance.
    /// Duplicate slots are applied in input order, with the last update winning.
    /// Submit previous queries before mutation, then rebuild instances before
    /// querying the changed scene. This does not wait for a GPU fence.
    /// # Errors
    /// Rejects any invalid slot, custom index or transform without partial changes.
    pub fn set_instances(&mut self, updates: &[RayInstanceUpdate]) -> Result<(), RaySceneError> {
        for update in updates {
            if update.index >= self.instance_capacity || update.custom_index > 0x00ff_ffff {
                return Err(RaySceneError::Capacity);
            }
            validate_transform(update.transform)?;
        }
        for update in updates {
            self.instances[update.index as usize] = Some(*update);
            self.tlas[update.index as usize] = Some(wgpu::TlasInstance::new(
                &self.blas,
                update.transform,
                update.custom_index,
                update.mask,
            ));
        }
        Ok(())
    }

    /// Removes an instance; rebuild TLAS before querying the changed scene.
    /// # Errors
    /// Rejects an out-of-capacity slot before mutation.
    pub fn remove_instance(&mut self, index: u32) -> Result<(), RaySceneError> {
        if index >= self.instance_capacity {
            return Err(RaySceneError::Capacity);
        }
        self.tlas[index as usize] = None;
        self.instances[index as usize] = None;
        Ok(())
    }

    /// Rebuilds only the TLAS after transform changes; the BLAS must already be built.
    pub fn build_instances(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.build_acceleration_structures(
            std::iter::empty::<&wgpu::BlasBuildEntry<'_>>(),
            [&self.tlas],
        );
    }

    /// Creates one opaque triangle BLAS and one identity instance TLAS.
    /// Device feature `EXPERIMENTAL_RAY_QUERY` must already be enabled by the host.
    /// # Errors
    /// Rejects unsupported devices, malformed geometry or insufficient device limits.
    pub fn new(device: &wgpu::Device, vertices: &[[f32; 3]]) -> Result<Self, RaySceneError> {
        Self::with_instance_capacity(device, vertices, 1)
    }

    /// Creates a shared triangle BLAS and a bounded instance TLAS, initially with slot zero active.
    /// # Errors
    /// Rejects unsupported features, malformed geometry or capacity beyond device limits.
    pub fn with_instance_capacity(
        device: &wgpu::Device,
        vertices: &[[f32; 3]],
        capacity: u32,
    ) -> Result<Self, RaySceneError> {
        Self::create_geometry(device, vertices, None, capacity)
    }

    /// Build an opaque indexed triangle BLAS from the scene's validated mesh.
    /// Shared vertices remain shared in GPU BLAS input. Materials/alpha are not
    /// used for intersection filtering; this path treats all triangles as opaque.
    /// # Errors
    /// Rejects unsupported ray devices or insufficient geometry/instance limits.
    pub fn from_scene_mesh(
        device: &wgpu::Device,
        mesh: &crate::SceneMesh,
        capacity: u32,
    ) -> Result<Self, RaySceneError> {
        let vertices: Vec<_> = mesh
            .vertices()
            .iter()
            .map(|vertex| vertex.position)
            .collect();
        Self::create_geometry(device, &vertices, Some(mesh.indices()), capacity)
    }

    fn create_geometry(
        device: &wgpu::Device,
        vertices: &[[f32; 3]],
        indices: Option<&[u32]>,
        capacity: u32,
    ) -> Result<Self, RaySceneError> {
        if !device
            .features()
            .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
        {
            return Err(RaySceneError::Unsupported);
        }
        if vertices.is_empty()
            || (indices.is_none() && !vertices.len().is_multiple_of(3))
            || indices.is_some_and(|indices| {
                indices.is_empty()
                    || !indices.len().is_multiple_of(3)
                    || indices
                        .iter()
                        .any(|&index| index as usize >= vertices.len())
            })
            || vertices.iter().flatten().any(|value| !value.is_finite())
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        let count = u32::try_from(vertices.len()).map_err(|_| RaySceneError::Capacity)?;
        let index_count = indices
            .map(|indices| u32::try_from(indices.len()))
            .transpose()
            .map_err(|_| RaySceneError::Capacity)?;
        let limits = device.limits();
        if index_count.unwrap_or(count) / 3 > limits.max_blas_primitive_count
            || limits.max_blas_geometry_count == 0
            || capacity == 0
            || capacity > limits.max_tlas_instance_count
            || u64::from(index_count.unwrap_or(0)) * 4 > limits.max_buffer_size
            || u64::from(count) * 12 > limits.max_buffer_size
        {
            return Err(RaySceneError::Capacity);
        }
        let geometry = wgpu::BlasTriangleGeometrySizeDescriptor {
            vertex_format: wgpu::VertexFormat::Float32x3,
            vertex_count: count,
            index_format: indices.map(|_| wgpu::IndexFormat::Uint32),
            index_count,
            flags: wgpu::AccelerationStructureGeometryFlags::OPAQUE,
        };
        let blas = device.create_blas(
            &wgpu::CreateBlasDescriptor {
                label: Some("scene triangle BLAS"),
                flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
                update_mode: wgpu::AccelerationStructureUpdateMode::Build,
            },
            wgpu::BlasGeometrySizeDescriptors::Triangles {
                descriptors: vec![geometry.clone()],
            },
        );
        let mut tlas = device.create_tlas(&wgpu::CreateTlasDescriptor {
            label: Some("scene TLAS"),
            max_instances: capacity,
            flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
            update_mode: wgpu::AccelerationStructureUpdateMode::Build,
        });
        tlas[0] = Some(wgpu::TlasInstance::new(
            &blas,
            [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            0,
            255,
        ));
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ray triangles"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::BLAS_INPUT,
        });
        let indices = indices.map(|indices| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ray triangle indices"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::BLAS_INPUT,
            })
        });
        let mut instances = vec![None; capacity as usize];
        instances[0] = Some(RayInstanceUpdate {
            index: 0,
            transform: [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            custom_index: 0,
            mask: 255,
        });
        Ok(Self {
            device: device.clone(),
            geometry_revision: 0,
            instances,
            indices,
            vertices,
            geometry,
            blas,
            tlas,
            instance_capacity: capacity,
        })
    }
    /// Replace indexed geometry while preserving every active/removed instance,
    /// transform, mask and custom index. Upload/validation completes before swap.
    /// Submit previous queries first; call `build` for the replacement before querying.
    /// A TLAS-only `build_instances` cannot build its newly allocated BLAS.
    /// Recreate consumer bind groups/jobs: previous bindings retain the old TLAS.
    /// This performs no GPU fence wait and does not reset application history.
    /// # Errors
    /// Rejects invalid geometry or device capacity without changing this scene.
    pub fn replace_scene_mesh(&mut self, mesh: &crate::SceneMesh) -> Result<(), RaySceneError> {
        let revision = self
            .geometry_revision
            .checked_add(1)
            .ok_or(RaySceneError::Capacity)?;
        let mut replacement = Self::from_scene_mesh(&self.device, mesh, self.instance_capacity)?;
        for index in 0..self.instance_capacity {
            replacement.remove_instance(index)?;
        }
        let active: Vec<_> = self.instances.iter().flatten().copied().collect();
        replacement.set_instances(&active)?;
        replacement.geometry_revision = revision;
        *self = replacement;
        Ok(())
    }

    /// Owner-local identity for acceleration-structure consumer binding caches.
    /// Increments only after successful geometry replacement. Instance transforms,
    /// masks and removal keep the same TLAS binding and do not increment it.
    /// Different `RayScene` owners may have equal revisions; retain owner identity
    /// alongside this value. Reset temporal history separately on content changes.
    #[must_use]
    pub const fn geometry_revision(&self) -> u64 {
        self.geometry_revision
    }

    /// Encode before any pass using this scene. Build and query may share an encoder.
    pub fn build(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.build_acceleration_structures(
            [&wgpu::BlasBuildEntry {
                blas: &self.blas,
                geometry: wgpu::BlasGeometries::TriangleGeometries(vec![
                    wgpu::BlasTriangleGeometry {
                        size: &self.geometry,
                        vertex_buffer: &self.vertices,
                        first_vertex: 0,
                        vertex_stride: 12,
                        index_buffer: self.indices.as_ref(),
                        first_index: self.indices.as_ref().map(|_| 0),
                        transform_buffer: None,
                        transform_buffer_offset: None,
                    },
                ]),
            }],
            [&self.tlas],
        );
    }
    #[must_use]
    pub fn binding(&self) -> wgpu::BindingResource<'_> {
        self.tlas.as_binding()
    }
    /// Primitive count shared by every instance of this triangle BLAS.
    #[must_use]
    pub fn triangle_count(&self) -> u32 {
        self.geometry
            .index_count
            .unwrap_or(self.geometry.vertex_count)
            / 3
    }
}

fn validate_transform(m: [f32; 12]) -> Result<(), RaySceneError> {
    let linear =
        glam::Mat3::from_cols_array(&[m[0], m[4], m[8], m[1], m[5], m[9], m[2], m[6], m[10]]);
    let determinant = linear.determinant();
    if m.iter().any(|value| !value.is_finite())
        || !determinant.is_finite()
        || determinant.abs() <= 0.0
    {
        return Err(RaySceneError::InvalidTransform);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn affine_instance_transform_validation() {
        let translated = [1., 0., 0., 4., 0., 2., 0., -3., 0., 0., -1., 2.];
        assert!(validate_transform(translated).is_ok());
        assert!(validate_transform([0.; 12]).is_err());
        let mut invalid = translated;
        invalid[3] = f32::NAN;
        assert!(validate_transform(invalid).is_err());
        invalid = translated;
        invalid[0] = f32::MAX;
        assert!(validate_transform(invalid).is_err());
    }
    #[test]
    fn ordinary_device_returns_explicit_unsupported() {
        let (device, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        assert!(matches!(
            RayScene::new(&device, &[[0.0; 3]; 3]),
            Err(RaySceneError::Unsupported)
        ));
    }
}

/// Owns segment inputs, output storage and one opaque visibility dispatch.
#[derive(Debug)]
pub struct RayVisibilityJob {
    pipeline: wgpu::ComputePipeline,
    bind: wgpu::BindGroup,
    output: wgpu::Buffer,
    count: u32,
}
/// Reusable visibility pipeline and its owning graphics device.
#[derive(Debug)]
pub struct RayVisibilityPipeline {
    device: wgpu::Device,
    pipeline: wgpu::ComputePipeline,
}
impl RayVisibilityPipeline {
    /// Compiles the opaque visibility pipeline once for this device.
    /// # Errors
    /// Rejects devices without enabled ray queries or adequate compute limits.
    pub fn new(device: &wgpu::Device) -> Result<Self, RaySceneError> {
        if !device
            .features()
            .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
        {
            return Err(RaySceneError::Unsupported);
        }
        if device.limits().max_compute_workgroup_size_x < 64
            || device.limits().max_compute_invocations_per_workgroup < 64
        {
            return Err(RaySceneError::Capacity);
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("opaque ray visibility"),
            source: wgpu::ShaderSource::Wgsl(RAY_VISIBILITY_SHADER.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("opaque ray visibility"),
            layout: None,
            module: &shader,
            entry_point: Some("cs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Ok(Self {
            device: device.clone(),
            pipeline,
        })
    }

    /// Creates independent segment/output storage using this cached pipeline.
    /// The scene must belong to the same device.
    /// # Errors
    /// Rejects invalid segments and buffer/dispatch limits before upload.
    pub fn create_job(
        &self,
        scene: &RayScene,
        segments: &[RaySegment],
    ) -> Result<RayVisibilityJob, RaySceneError> {
        let device = &self.device;
        scene.validate_device(device)?;
        for segment in segments {
            RaySegment::new(
                [
                    segment.origin_bias[0],
                    segment.origin_bias[1],
                    segment.origin_bias[2],
                ],
                [segment.target[0], segment.target[1], segment.target[2]],
                segment.origin_bias[3],
            )?;
        }
        let count = u32::try_from(segments.len()).map_err(|_| RaySceneError::Capacity)?;
        let limits = device.limits();
        let input_bytes = u64::from(count) * 32;
        let output_bytes = u64::from(count) * 4;
        if limits.max_compute_workgroup_size_x < 64
            || limits.max_compute_invocations_per_workgroup < 64
            || count == 0
            || input_bytes > limits.max_buffer_size
            || input_bytes > limits.max_storage_buffer_binding_size
            || output_bytes > limits.max_buffer_size
            || count.div_ceil(64) > limits.max_compute_workgroups_per_dimension
        {
            return Err(RaySceneError::Capacity);
        }
        let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ray visibility segments"),
            contents: bytemuck::cast_slice(segments),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ray visibility output"),
            size: output_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ray visibility job"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: scene.binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: input.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output.as_entire_binding(),
                },
            ],
        });
        Ok(RayVisibilityJob {
            pipeline: self.pipeline.clone(),
            bind,
            output,
            count,
        })
    }
}

impl RayVisibilityJob {
    /// Convenience constructor; reuse `RayVisibilityPipeline` for recurring work.
    /// # Errors
    /// Rejects unsupported devices, invalid segments or capacity overflow.
    pub fn new(
        device: &wgpu::Device,
        scene: &RayScene,
        segments: &[RaySegment],
    ) -> Result<Self, RaySceneError> {
        RayVisibilityPipeline::new(device)?.create_job(scene, segments)
    }

    /// Encode after scene acceleration builds, then submit before using outputs.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind, &[]);
        pass.dispatch_workgroups(self.count.div_ceil(64), 1, 1);
    }

    /// One u32 per input segment: zero blocked, one visible. No CPU mapping is implied.
    #[must_use]
    pub fn output(&self) -> &wgpu::Buffer {
        &self.output
    }
}

/// Portable opaque, double-sided triangle visibility fallback.
/// Cost is linear in segments times triangles; use a spatial index for large scenes.
/// Triangles must be in world coordinates, with three consecutive vertices per face.
/// # Errors
/// Rejects malformed geometry and invalid segments before intersection work.
#[allow(clippy::many_single_char_names)] // Conventional triangle intersection notation.
pub fn cpu_segment_visibility(
    vertices: &[[f32; 3]],
    segments: &[RaySegment],
) -> Result<Vec<u32>, RaySceneError> {
    if !vertices.len().is_multiple_of(3) || vertices.iter().flatten().any(|v| !v.is_finite()) {
        return Err(RaySceneError::InvalidGeometry);
    }
    let mut output = Vec::with_capacity(segments.len());
    for segment in segments {
        let origin = [
            segment.origin_bias[0],
            segment.origin_bias[1],
            segment.origin_bias[2],
        ];
        let destination = [segment.target[0], segment.target[1], segment.target[2]];
        RaySegment::new(origin, destination, segment.origin_bias[3])?;
        let origin = glam::Vec3::from_array(origin).as_dvec3();
        let delta = glam::Vec3::from_array(destination).as_dvec3() - origin;
        let distance = delta.length();
        let direction = delta / distance;
        let bias = f64::from(segment.origin_bias[3]);
        let blocked = vertices.chunks_exact(3).any(|triangle| {
            let a = glam::Vec3::from_array(triangle[0]).as_dvec3();
            let b = glam::Vec3::from_array(triangle[1]).as_dvec3();
            let c = glam::Vec3::from_array(triangle[2]).as_dvec3();
            let edge1 = b - a;
            let edge2 = c - a;
            let p = direction.cross(edge2);
            let determinant = edge1.dot(p);
            if determinant.abs() <= f64::MIN_POSITIVE {
                return false;
            }
            let inv = determinant.recip();
            let tvec = origin - a;
            let u = tvec.dot(p) * inv;
            if !(0.0..=1.0).contains(&u) {
                return false;
            }
            let q = tvec.cross(edge1);
            let v = direction.dot(q) * inv;
            if v < 0.0 || u + v > 1.0 {
                return false;
            }
            let t = edge2.dot(q) * inv;
            t >= bias && t <= distance - bias
        });
        output.push(u32::from(!blocked));
    }
    Ok(output)
}

#[cfg(test)]
mod cpu_visibility_tests {
    use super::*;
    #[test]
    fn fallback_is_double_sided_and_bounded_to_light() {
        let vertices = [[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]];
        let segments = [
            RaySegment::new([0., 0., 1.], [0., 0., -1.], 0.001).unwrap(),
            RaySegment::new([0., 0., -1.], [0., 0., 1.], 0.001).unwrap(),
            RaySegment::new([0., 0., 1.], [0., 0., 0.5], 0.001).unwrap(),
        ];
        assert_eq!(
            cpu_segment_visibility(&vertices, &segments).unwrap(),
            [0, 0, 1]
        );
        assert_eq!(cpu_segment_visibility(&[], &segments).unwrap(), [1, 1, 1]);
        assert!(cpu_segment_visibility(&vertices[..2], &segments).is_err());
        assert!(RaySegment::new([0.; 3], [0.; 3], 0.001).is_err());
        assert!(RaySegment::new([0.; 3], [1.; 3], -1.0).is_err());
    }
}
