//! Single point-light Lambertian contribution with opaque ray-traced visibility.
use crate::{RayScene, RaySceneError, RaySegment, RayVisibilityJob};
use wgpu::util::DeviceExt;

#[derive(Clone, Copy, Debug)]
pub struct PointLightSample {
    segment: RaySegment,
    unoccluded: [f32; 4],
}
impl PointLightSample {
    /// Sample a point light's radiant intensity (linear RGB, power per steradian).
    /// Computes diffuse / pi, N dot L and inverse-square attenuation. Diffuse
    /// reflectance must already exclude energy assigned to other material lobes.
    /// # Errors
    /// Rejects invalid geometry, reflectance/intensity, ray bias or HDR overflow.
    pub fn new(
        position: [f32; 3],
        normal: [f32; 3],
        diffuse: [f32; 3],
        light_position: [f32; 3],
        intensity: [f32; 3],
        bias: f32,
    ) -> Result<Self, RaySceneError> {
        let segment = RaySegment::new(position, light_position, bias)?;
        if diffuse
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            || intensity.iter().any(|v| !v.is_finite() || *v < 0.0)
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        let normal = glam::Vec3::from_array(normal)
            .try_normalize()
            .ok_or(RaySceneError::InvalidGeometry)?;
        let delta = glam::Vec3::from_array(light_position) - glam::Vec3::from_array(position);
        let squared_distance = delta.length_squared();
        let direction = delta
            .try_normalize()
            .ok_or(RaySceneError::InvalidGeometry)?;
        let rgb = glam::Vec3::from_array(diffuse)
            * glam::Vec3::from_array(intensity)
            * (normal.dot(direction).max(0.0) / (std::f32::consts::PI * squared_distance));
        if !rgb.is_finite() || rgb.max_element() > 65504.0 {
            return Err(RaySceneError::InvalidGeometry);
        }
        Ok(Self {
            segment,
            unoccluded: rgb.extend(1.0).to_array(),
        })
    }
}
#[derive(Debug)]
pub struct DirectLightingJob {
    visibility: RayVisibilityJob,
    pipeline: wgpu::ComputePipeline,
    bindings: wgpu::BindGroup,
    output: wgpu::Texture,
}
impl DirectLightingJob {
    /// Upload coherent surface/light samples and allocate an HDR lighting texture.
    /// Scene and samples must describe the same world coordinates on this device.
    /// # Errors
    /// Rejects unsupported ray queries, sample count/dimensions and storage limits.
    pub fn new(
        device: &wgpu::Device,
        scene: &RayScene,
        dimensions: [u32; 2],
        samples: &[PointLightSample],
    ) -> Result<Self, RaySceneError> {
        let [width, height] = dimensions;
        let count = width.checked_mul(height).ok_or(RaySceneError::Capacity)?;
        let limits = device.limits();
        if count == 0
            || usize::try_from(count).ok() != Some(samples.len())
            || width > limits.max_texture_dimension_2d
            || height > limits.max_texture_dimension_2d
            || width.div_ceil(8) > limits.max_compute_workgroups_per_dimension
            || height.div_ceil(8) > limits.max_compute_workgroups_per_dimension
            || limits.max_compute_workgroup_size_x < 8
            || limits.max_compute_workgroup_size_y < 8
            || limits.max_compute_invocations_per_workgroup < 64
            || limits.max_storage_buffers_per_shader_stage < 2
            || limits.max_storage_textures_per_shader_stage < 1
            || u64::from(count) * 16 > limits.max_storage_buffer_binding_size
        {
            return Err(RaySceneError::Capacity);
        }
        let segments: Vec<_> = samples.iter().map(|sample| sample.segment).collect();
        let visibility = RayVisibilityJob::new(device, scene, &segments)?;
        let radiance: Vec<_> = samples.iter().map(|sample| sample.unoccluded).collect();
        let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("unoccluded Lambertian point light"),
            contents: bytemuck::cast_slice(&radiance),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ray-shadowed HDR direct lighting"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ray-shadowed Lambertian light"),
            source: wgpu::ShaderSource::Wgsl(include_str!("direct_lighting.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ray-shadowed Lambertian light"),
            layout: None,
            module: &shader,
            entry_point: Some("cs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let view = output.create_view(&wgpu::TextureViewDescriptor::default());
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("direct lighting visibility"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: visibility.output().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: input.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
            ],
        });
        Ok(Self {
            visibility,
            pipeline,
            bindings,
            output,
        })
    }
    /// Encode after scene build; visibility and HDR shading run in this order.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        self.visibility.encode(encoder);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lambertian_inverse_square_and_back_facing_light() {
        let sample = |z, normal| {
            PointLightSample::new([0.0; 3], normal, [0.5; 3], [0.0, 0.0, z], [8.0; 3], 0.001)
                .unwrap()
        };
        let near = sample(1.0, [0.0, 0.0, 2.0]);
        let far = sample(2.0, [0.0, 0.0, 1.0]);
        for channel in 0..3 {
            assert!((near.unoccluded[channel] - 4.0 / std::f32::consts::PI).abs() < 0.000_001);
            assert!((far.unoccluded[channel] * 4.0 - near.unoccluded[channel]).abs() < 0.000_001);
        }
        assert_eq!(
            sample(1.0, [0.0, 0.0, -1.0]).unoccluded,
            [0.0, 0.0, 0.0, 1.0]
        );
    }
    #[test]
    fn invalid_material_light_and_geometry_are_rejected() {
        for invalid in [f32::NAN, f32::INFINITY, -0.1] {
            assert!(
                PointLightSample::new(
                    [0.0; 3],
                    [0.0, 0.0, 1.0],
                    [0.5; 3],
                    [0.0, 0.0, 1.0],
                    [invalid; 3],
                    0.001
                )
                .is_err()
            );
        }
        assert!(
            PointLightSample::new(
                [0.0; 3],
                [0.0; 3],
                [0.5; 3],
                [0.0, 0.0, 1.0],
                [1.0; 3],
                0.001
            )
            .is_err()
        );
        assert!(
            PointLightSample::new(
                [0.0; 3],
                [0.0, 0.0, 1.0],
                [1.1; 3],
                [0.0, 0.0, 1.0],
                [1.0; 3],
                0.001
            )
            .is_err()
        );
        assert!(
            PointLightSample::new(
                [0.0; 3],
                [0.0, 0.0, 1.0],
                [1.0; 3],
                [0.0, 0.0, 1.0],
                [f32::MAX; 3],
                0.001
            )
            .is_err()
        );
    }
}
