//! Closest-hit mirror rays measured from their primary world-space surfaces.
use crate::{RayScene, RaySceneError};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpecularRay {
    origin_bias: [f32; 4],
    direction_distance: [f32; 4],
}
impl SpecularRay {
    /// Reflect the camera-to-surface direction around the normalized world normal.
    /// Bias is t-min only: reported distance still starts at the primary surface.
    /// # Errors
    /// Rejects nonfinite/degenerate geometry and invalid bias/range.
    pub fn from_surface(
        position: [f32; 3],
        normal: [f32; 3],
        camera: [f32; 3],
        bias: f32,
        maximum_distance: f32,
    ) -> Result<Self, RaySceneError> {
        let position = glam::Vec3::from_array(position);
        let camera = glam::Vec3::from_array(camera);
        if !position.is_finite()
            || !camera.is_finite()
            || !bias.is_finite()
            || bias <= 0.0
            || !maximum_distance.is_finite()
            || maximum_distance <= bias
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        let normal = glam::Vec3::from_array(normal)
            .try_normalize()
            .ok_or(RaySceneError::InvalidGeometry)?;
        let incident = (position - camera)
            .try_normalize()
            .ok_or(RaySceneError::InvalidGeometry)?;
        let direction = (incident - 2.0 * incident.dot(normal) * normal)
            .try_normalize()
            .ok_or(RaySceneError::InvalidGeometry)?;
        Ok(Self {
            origin_bias: position.extend(bias).to_array(),
            direction_distance: direction.extend(maximum_distance).to_array(),
        })
    }
}
/// Coherent rough reflection ray and Monte Carlo throughput.
#[derive(Clone, Copy, Debug)]
pub struct GgxSurfaceSample {
    ray: SpecularRay,
    throughput: [f32; 4],
}
impl GgxSurfaceSample {
    /// Sample a rough surface. None is a zero contribution, without retrying.
    /// # Errors
    /// Rejects invalid surface/material/random/range or unrepresentable weights.
    pub fn new(
        material: &crate::ReconstructionMaterial,
        position: [f32; 3],
        normal: [f32; 3],
        camera: [f32; 3],
        uniform: [f32; 2],
        interval: [f32; 2],
    ) -> Result<Option<Self>, RaySceneError> {
        let mut ray =
            SpecularRay::from_surface(position, normal, camera, interval[0], interval[1])?;
        let view = (glam::Vec3::from_array(camera) - glam::Vec3::from_array(position)).to_array();
        let Some(sample) = material
            .sample_ggx_reflection(normal, view, uniform)
            .map_err(|_| RaySceneError::InvalidGeometry)?
        else {
            return Ok(None);
        };
        if sample
            .throughput
            .iter()
            .any(|v| *v < 0.0 || *v > f64::from(f32::MAX / 65536.0))
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        ray.direction_distance[..3].copy_from_slice(&sample.direction);
        #[allow(clippy::cast_possible_truncation)]
        let weight = sample.throughput.map(|v| v as f32);
        Ok(Some(Self {
            ray,
            throughput: [weight[0], weight[1], weight[2], 1.0],
        }))
    }
}
/// An ideal-mirror ray and its material weight, derived from the same surface.
#[derive(Clone, Copy, Debug)]
pub struct MirrorSurfaceSample {
    ray: SpecularRay,
    throughput: [f32; 4],
}
impl MirrorSurfaceSample {
    /// Construct a reflection-only delta-mirror sample in world coordinates.
    /// # Errors
    /// Rejects rough materials, invalid directions/positions or ray intervals.
    pub fn new(
        material: &crate::ReconstructionMaterial,
        position: [f32; 3],
        normal: [f32; 3],
        camera: [f32; 3],
        bias: f32,
        maximum_distance: f32,
    ) -> Result<Self, RaySceneError> {
        let ray = SpecularRay::from_surface(position, normal, camera, bias, maximum_distance)?;
        let to_camera =
            (glam::Vec3::from_array(camera) - glam::Vec3::from_array(position)).to_array();
        let throughput = material
            .mirror_throughput(normal, to_camera)
            .map_err(|_| RaySceneError::InvalidGeometry)?;
        Ok(Self { ray, throughput })
    }
}
#[derive(Debug)]
pub struct SpecularDistancePipeline {
    radiance_format: wgpu::TextureFormat,
    device: wgpu::Device,
    pipeline: wgpu::ComputePipeline,
}
#[derive(Debug)]
pub struct SpecularDistanceJob {
    pipeline: wgpu::ComputePipeline,
    bindings: wgpu::BindGroup,
    output: wgpu::Texture,
    radiance: wgpu::Texture,
}
impl SpecularDistancePipeline {
    /// Compile closest-hit ray queries once. Requires enabled experimental ray queries.
    /// # Errors
    /// Rejects unsupported features or insufficient compute limits.
    pub fn new(device: &wgpu::Device) -> Result<Self, RaySceneError> {
        Self::with_format(device, wgpu::TextureFormat::Rgba16Float)
    }
    /// Wider HDR output for Monte Carlo weights above one.
    /// # Errors
    /// Preserves ray-query and compute capability checks.
    pub fn new_hdr(device: &wgpu::Device) -> Result<Self, RaySceneError> {
        Self::with_format(device, wgpu::TextureFormat::Rgba32Float)
    }
    fn with_format(
        device: &wgpu::Device,
        radiance_format: wgpu::TextureFormat,
    ) -> Result<Self, RaySceneError> {
        if !device
            .features()
            .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
        {
            return Err(RaySceneError::Unsupported);
        }
        let limits = device.limits();
        if limits.max_compute_workgroup_size_x < 8
            || limits.max_compute_workgroup_size_y < 8
            || limits.max_compute_invocations_per_workgroup < 64
            || limits.max_storage_buffers_per_shader_stage < 3
            || limits.max_storage_textures_per_shader_stage < 2
        {
            return Err(RaySceneError::Capacity);
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("specular closest-hit distance"),
            source: wgpu::ShaderSource::Wgsl(
                if radiance_format == wgpu::TextureFormat::Rgba32Float {
                    include_str!("reflection.wgsl").replace("rgba16float", "rgba32float")
                } else {
                    include_str!("reflection.wgsl").to_owned()
                }
                .into(),
            ),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("specular distance pipeline"),
            layout: None,
            module: &shader,
            entry_point: Some("cs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Ok(Self {
            radiance_format,
            device: device.clone(),
            pipeline,
        })
    }
    /// One world-space ray per output pixel, in row-major order. Scene must share
    /// this device and be built before dispatch. Misses write zero; consumers must
    /// match that convention. Only opaque triangles are handled by this scene path.
    /// # Errors
    /// Rejects ray count/dimensions, invalid ray bytes and buffer/dispatch limits.
    pub fn create_job(
        &self,
        scene: &RayScene,
        width: u32,
        height: u32,
        rays: &[SpecularRay],
    ) -> Result<SpecularDistanceJob, RaySceneError> {
        self.create_job_inner(scene, width, height, rays, None, None)
    }
    /// Record reflected incident radiance from emissive opaque triangles.
    /// One linear RGB radiance entry is required per BLAS primitive; all instances
    /// share that table. Misses write black. This is ideal-mirror incoming radiance,
    /// not final surface shading: apply the primary material's BSDF separately.
    /// # Errors
    /// Rejects table length, nonfinite/negative or binary16-overflowing emission,
    /// plus all ray-job dimensions/range/storage validation errors.
    pub fn create_radiance_job(
        &self,
        scene: &RayScene,
        width: u32,
        height: u32,
        rays: &[SpecularRay],
        emission: &[[f32; 4]],
    ) -> Result<SpecularDistanceJob, RaySceneError> {
        if usize::try_from(scene.triangle_count()).ok() != Some(emission.len())
            || emission.iter().any(|entry| {
                entry[..3]
                    .iter()
                    .any(|v| !v.is_finite() || !(0.0..=65504.0).contains(v))
            })
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        self.create_job_inner(scene, width, height, rays, Some(emission), None)
    }
    /// Apply per-pixel primary BSDF throughput to reflected incident emission.
    /// Throughput must include the sampling weight (BSDF * cosine / PDF for
    /// sampled lobes, Fresnel reflectance for delta mirrors). It is not the RR
    /// specular-albedo guide. This single-bounce path omits other lighting terms.
    /// # Errors
    /// Rejects invalid emission, throughput count, negative/nonfinite weights,
    /// or weights that can overflow the output. `new_hdr` permits weights above
    /// one, conservatively bounded by `f32::MAX/65536`; the binary16 path caps at one.
    pub fn create_weighted_radiance_job(
        &self,
        scene: &RayScene,
        dimensions: [u32; 2],
        rays: &[SpecularRay],
        emission: &[[f32; 4]],
        throughput: &[[f32; 4]],
    ) -> Result<SpecularDistanceJob, RaySceneError> {
        if usize::try_from(scene.triangle_count()).ok() != Some(emission.len())
            || emission.iter().any(|v| {
                v[..3]
                    .iter()
                    .any(|c| !c.is_finite() || !(0.0..=65504.0).contains(c))
            })
            || throughput.len() != rays.len()
            || throughput.iter().any(|v| {
                v[..3]
                    .iter()
                    .any(|c| !c.is_finite() || !(0.0..=self.maximum_weight()).contains(c))
            })
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        let weights: Vec<_> = throughput.iter().map(|w| [w[0], w[1], w[2], 1.0]).collect();
        self.create_job_inner(
            scene,
            dimensions[0],
            dimensions[1],
            rays,
            Some(emission),
            Some(&weights),
        )
    }
    /// Derive ray and BSDF arrays together from coherent primary mirror samples.
    /// # Errors
    /// Rejects count/dimension/storage limits or invalid emission table values.
    pub fn create_mirror_job(
        &self,
        scene: &RayScene,
        dimensions: [u32; 2],
        samples: &[MirrorSurfaceSample],
        emission: &[[f32; 4]],
    ) -> Result<SpecularDistanceJob, RaySceneError> {
        let rays: Vec<_> = samples.iter().map(|sample| sample.ray).collect();
        let throughput: Vec<_> = samples.iter().map(|sample| sample.throughput).collect();
        self.create_weighted_radiance_job(scene, dimensions, &rays, emission, &throughput)
    }
    /// Trace one GGX sample per pixel, retaining null samples as black/zero distance.
    /// Requires the wide HDR constructor. No acceptance renormalization is applied.
    /// # Errors
    /// Rejects binary16 output, invalid emission and dimensions/capacity.
    pub fn create_ggx_job(
        &self,
        scene: &RayScene,
        dimensions: [u32; 2],
        samples: &[Option<GgxSurfaceSample>],
        emission: &[[f32; 4]],
    ) -> Result<SpecularDistanceJob, RaySceneError> {
        if self.radiance_format != wgpu::TextureFormat::Rgba32Float {
            return Err(RaySceneError::Unsupported);
        }
        if usize::try_from(scene.triangle_count()).ok() != Some(emission.len())
            || emission.iter().any(|v| {
                v[..3]
                    .iter()
                    .any(|c| !c.is_finite() || !(0.0..=65504.0).contains(c))
            })
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        let placeholder = SpecularRay {
            origin_bias: [0.0, 0.0, 0.0, 0.001],
            direction_distance: [0.0, 0.0, 1.0, 1.0],
        };
        let rays: Vec<_> = samples
            .iter()
            .map(|s| s.map_or(placeholder, |s| s.ray))
            .collect();
        let weights: Vec<_> = samples
            .iter()
            .map(|s| s.map_or([0.0; 4], |s| s.throughput))
            .collect();
        self.create_job_inner(
            scene,
            dimensions[0],
            dimensions[1],
            &rays,
            Some(emission),
            Some(&weights),
        )
    }
    fn maximum_weight(&self) -> f32 {
        if self.radiance_format == wgpu::TextureFormat::Rgba32Float {
            f32::MAX / 65536.0
        } else {
            1.0
        }
    }
    fn create_job_inner(
        &self,
        scene: &RayScene,
        width: u32,
        height: u32,
        rays: &[SpecularRay],
        emission: Option<&[[f32; 4]]>,
        throughput: Option<&[[f32; 4]]>,
    ) -> Result<SpecularDistanceJob, RaySceneError> {
        scene.validate_device(&self.device)?;
        let count = width.checked_mul(height).ok_or(RaySceneError::Capacity)?;
        let limits = self.device.limits();
        let bytes = u64::from(count) * 32;
        if width == 0
            || height == 0
            || usize::try_from(count).ok() != Some(rays.len())
            || width > limits.max_texture_dimension_2d
            || height > limits.max_texture_dimension_2d
            || width.div_ceil(8) > limits.max_compute_workgroups_per_dimension
            || height.div_ceil(8) > limits.max_compute_workgroups_per_dimension
            || bytes > limits.max_buffer_size
            || bytes > limits.max_storage_buffer_binding_size
        {
            return Err(RaySceneError::Capacity);
        }
        validate_rays(rays)?;
        let input = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("world specular rays"),
                contents: bytemuck::cast_slice(rays),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let output = ray_texture(&self.device, width, height, wgpu::TextureFormat::R32Float);
        let view = output.create_view(&wgpu::TextureViewDescriptor::default());
        let emission = emission.unwrap_or(&[[0.0; 4]]);
        let emission_bytes =
            u64::try_from(emission.len()).map_err(|_| RaySceneError::Capacity)? * 16;
        if emission_bytes > limits.max_buffer_size
            || emission_bytes > limits.max_storage_buffer_binding_size
        {
            return Err(RaySceneError::Capacity);
        }
        let emission_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("primitive emitted radiance"),
                contents: bytemuck::cast_slice(emission),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let radiance = ray_texture(&self.device, width, height, self.radiance_format);
        let default_throughput = [[1.0; 4]];
        let weights = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("primary BSDF throughput"),
                contents: bytemuck::cast_slice(throughput.unwrap_or(&default_throughput)),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let radiance_view = radiance.create_view(&wgpu::TextureViewDescriptor::default());
        let bindings = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("specular ray job"),
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
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: emission_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&radiance_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: weights.as_entire_binding(),
                },
            ],
        });
        Ok(SpecularDistanceJob {
            pipeline: self.pipeline.clone(),
            bindings,
            output,
            radiance,
        })
    }
}
fn ray_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("reflected ray output"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
fn validate_rays(rays: &[SpecularRay]) -> Result<(), RaySceneError> {
    for ray in rays {
        let origin = glam::Vec3::new(ray.origin_bias[0], ray.origin_bias[1], ray.origin_bias[2]);
        let direction = glam::Vec3::new(
            ray.direction_distance[0],
            ray.direction_distance[1],
            ray.direction_distance[2],
        );
        if !origin.is_finite()
            || !direction.is_finite()
            || (direction.length_squared() - 1.0).abs() > 0.0001
            || !ray.origin_bias[3].is_finite()
            || ray.origin_bias[3] <= 0.0
            || !ray.direction_distance[3].is_finite()
            || ray.direction_distance[3] <= ray.origin_bias[3]
        {
            return Err(RaySceneError::InvalidGeometry);
        }
    }
    Ok(())
}
impl SpecularDistanceJob {
    /// Dispatch after BLAS/TLAS build on the same ordered submission stream.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bindings, &[]);
        pass.dispatch_workgroups(
            self.output.width().div_ceil(8),
            self.output.height().div_ceil(8),
            1,
        );
    }
    #[must_use]
    pub fn output(&self) -> &wgpu::Texture {
        &self.output
    }
    /// HDR reflected radiance. Unweighted jobs return incoming emission; weighted
    /// jobs include the caller-provided primary BSDF throughput.
    #[must_use]
    pub fn incident_radiance(&self) -> &wgpu::Texture {
        &self.radiance
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reflected_direction_and_surface_distance_origin() {
        let ray = SpecularRay::from_surface(
            [0.0, 0.0, 2.0],
            [0.0, 0.0, 2.0],
            [0.0, 0.0, 4.0],
            0.01,
            10.0,
        )
        .unwrap();
        assert_eq!(ray.origin_bias, [0.0, 0.0, 2.0, 0.01]);
        assert_eq!(ray.direction_distance, [0.0, 0.0, 1.0, 10.0]);
    }
    #[test]
    fn invalid_geometry_and_ranges_fail() {
        for (normal, camera, bias, range) in [
            ([0.0; 3], [0.0, 0.0, 4.0], 0.01, 10.0),
            ([0.0, 0.0, 1.0], [0.0; 3], 0.01, 10.0),
            ([0.0, 0.0, 1.0], [0.0, 0.0, 4.0], 0.0, 10.0),
            ([0.0, 0.0, 1.0], [0.0, 0.0, 4.0], 0.1, 0.1),
            ([f32::NAN; 3], [0.0, 0.0, 4.0], 0.01, 10.0),
        ] {
            assert!(SpecularRay::from_surface([0.0; 3], normal, camera, bias, range).is_err());
        }
    }
}
