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

/// Computes the 0-based logarithmic depth slice index `[0, num_slices - 1]` for a clustered
/// forward/deferred light grid (`slice = floor(log(z / z_near) / log(z_far / z_near) * num_slices)`).
///
/// Returns `None` if `z_near <= 0.0`, `z_far <= z_near`, `num_slices == 0`, or inputs are non-finite.
#[must_use]
pub fn cluster_depth_slice(
    view_z: f32,
    z_near: f32,
    z_far: f32,
    num_slices: u32,
) -> Option<u32> {
    if !view_z.is_finite()
        || !z_near.is_finite()
        || !z_far.is_finite()
        || z_near <= 0.0
        || z_far <= z_near
        || num_slices == 0
    {
        return None;
    }
    let clamped_z = view_z.clamp(z_near, z_far);
    let log_ratio = (clamped_z / z_near).ln() / (z_far / z_near).ln();
    let raw = (log_ratio * (num_slices as f32)).floor().max(0.0) as u32;
    Some(raw.min(num_slices - 1))
}

/// Evaluates the qualitative Oren-Nayar rough diffuse BRDF `(rho / pi) * (A + B * max(0, cos(phi_i - phi_r)) * sin(alpha) * tan(beta))`
/// for rough matte surfaces such as concrete, plaster, sand, and stone.
#[must_use]
pub fn oren_nayar_diffuse_reflectance(
    n: [f32; 3],
    v: [f32; 3],
    l: [f32; 3],
    albedo: f32,
    roughness: f32,
) -> f32 {
    if n.iter().chain(&v).chain(&l).any(|c| !c.is_finite())
        || !albedo.is_finite()
        || !roughness.is_finite()
    {
        return 0.0;
    }
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let nl = dot(n, l).clamp(0.0, 1.0);
    let nv = dot(n, v).clamp(0.0, 1.0);
    if nl <= 0.0 || nv <= 0.0 {
        return 0.0;
    }
    let sigma = roughness.clamp(0.0, 1.0) * std::f32::consts::FRAC_PI_2;
    let sigma2 = sigma * sigma;
    let a = 1.0 - 0.5 * (sigma2 / (sigma2 + 0.33));
    let b = 0.45 * (sigma2 / (sigma2 + 0.09));
    let lv = dot(l, v);
    let s = lv - nl * nv;
    let t = if s > 0.0 {
        nl.max(nv).max(1e-5)
    } else {
        1.0
    };
    let rho = albedo.clamp(0.0, 1.0);
    (rho * std::f32::consts::FRAC_1_PI * (a + b * s.max(0.0) / t)).max(0.0)
}

/// Evaluates Lommel-Seeliger / Hapke opposition surge retroreflective BRDF for dusty regolith,
/// powdery snow, and dry soil: `1 / (ndotl + ndotv) * (1 + B0 / (1 + tan(phase / 2) / h))`.
#[must_use]
pub fn lunar_dust_retroreflection(
    ndotl: f32,
    ndotv: f32,
    cos_phase: f32,
    backscatter_b0: f32,
    width_h: f32,
) -> f32 {
    if !ndotl.is_finite()
        || !ndotv.is_finite()
        || !cos_phase.is_finite()
        || !backscatter_b0.is_finite()
        || !width_h.is_finite()
    {
        return 0.0;
    }
    let nl = ndotl.clamp(0.0, 1.0);
    let nv = ndotv.clamp(0.0, 1.0);
    if nl <= 0.0 || nv <= 0.0 {
        return 0.0;
    }
    let lommel = nl / (nl + nv).max(1e-4);
    let c = cos_phase.clamp(-1.0, 1.0);
    // tan(phase / 2) = sqrt((1 - cos(phase)) / (1 + cos(phase)))
    let tan_half = ((1.0 - c).max(0.0) / (1.0 + c).max(1e-6)).sqrt();
    let h = width_h.max(1e-4);
    let b0 = backscatter_b0.max(0.0);
    let surge = 1.0 + b0 / (1.0 + tan_half / h);
    (lommel * surge).max(0.0)
}

/// Computes Karis (2013) Most Representative Point (MRP) direction vector from the shading point
/// toward a spherical area light of `sphere_radius` around `to_light_center` along `reflection_dir`.
#[must_use]
pub fn sphere_light_mrt_representative_point(
    reflection_dir: [f32; 3],
    to_light_center: [f32; 3],
    sphere_radius: f32,
) -> [f32; 3] {
    if reflection_dir.iter().chain(&to_light_center).any(|c| !c.is_finite())
        || !sphere_radius.is_finite()
    {
        return [0.0, 0.0, 1.0];
    }
    let r_len = (reflection_dir[0].powi(2) + reflection_dir[1].powi(2) + reflection_dir[2].powi(2)).sqrt();
    if r_len <= 1e-6 {
        return to_light_center;
    }
    let r = [
        reflection_dir[0] / r_len,
        reflection_dir[1] / r_len,
        reflection_dir[2] / r_len,
    ];
    let l = to_light_center;
    let l_dot_r = l[0] * r[0] + l[1] * r[1] + l[2] * r[2];
    let center_to_ray = [
        l_dot_r * r[0] - l[0],
        l_dot_r * r[1] - l[1],
        l_dot_r * r[2] - l[2],
    ];
    let dist_to_ray = (center_to_ray[0].powi(2) + center_to_ray[1].powi(2) + center_to_ray[2].powi(2)).sqrt();
    let rad = sphere_radius.max(0.0);
    if dist_to_ray <= 1e-6 || rad <= 0.0 {
        return l;
    }
    let t = (rad / dist_to_ray).clamp(0.0, 1.0);
    [
        l[0] + center_to_ray[0] * t,
        l[1] + center_to_ray[1] * t,
        l[2] + center_to_ray[2] * t,
    ]
}

/// Evaluates the horizon-clipped linear tube/capsule light irradiance factor between endpoints `seg_a`
/// and `seg_b` at shading point `p` with unit normal `n` (Picott / Karis analytical line-light form factor).
#[must_use]
pub fn capsule_light_irradiance_factor(
    p: [f32; 3],
    n: [f32; 3],
    seg_a: [f32; 3],
    seg_b: [f32; 3],
) -> f32 {
    if p.iter()
        .chain(&n)
        .chain(&seg_a)
        .chain(&seg_b)
        .any(|c| !c.is_finite())
    {
        return 0.0;
    }
    let mut l0 = [seg_a[0] - p[0], seg_a[1] - p[1], seg_a[2] - p[2]];
    let mut l1 = [seg_b[0] - p[0], seg_b[1] - p[1], seg_b[2] - p[2]];
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let mut h0 = dot(n, l0);
    let mut h1 = dot(n, l1);
    if h0 <= 0.0 && h1 <= 0.0 {
        return 0.0;
    }
    // Horizon clipping when one endpoint lies below the tangent plane.
    if h0 <= 0.0 {
        let t = (-h0 / (h1 - h0).max(1e-6)).clamp(0.0, 1.0);
        l0 = [
            l0[0] + t * (l1[0] - l0[0]),
            l0[1] + t * (l1[1] - l0[1]),
            l0[2] + t * (l1[2] - l0[2]),
        ];
        h0 = dot(n, l0).max(0.0);
    } else if h1 <= 0.0 {
        let t = (-h1 / (h0 - h1).max(1e-6)).clamp(0.0, 1.0);
        l1 = [
            l1[0] + t * (l0[0] - l1[0]),
            l1[1] + t * (l0[1] - l1[1]),
            l1[2] + t * (l0[2] - l1[2]),
        ];
        h1 = dot(n, l1).max(0.0);
    }
    let d0 = dot(l0, l0).max(1e-6).sqrt();
    let d1 = dot(l1, l1).max(1e-6).sqrt();
    let cos_gamma = (dot(l0, l1) / (d0 * d1)).clamp(-0.999_99, 0.999_99);
    let gamma = cos_gamma.acos();
    let sin_gamma = (1.0 - cos_gamma * cos_gamma).max(1e-6).sqrt();
    let weight = if gamma < 1e-3 {
        1.0
    } else {
        gamma / sin_gamma
    };
    ((h0 / d0 + h1 / d1) * weight / (2.0 * std::f32::consts::PI * d0 * d1)).max(0.0)
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
    #[test]
    fn logarithmic_cluster_depth_slice_partitions_view_frustum() {
        assert_eq!(cluster_depth_slice(0.1, 0.1, 100.0, 24), Some(0));
        assert_eq!(cluster_depth_slice(100.0, 0.1, 100.0, 24), Some(23));
        // Geometric mean sqrt(0.1 * 100.0) = sqrt(10) ≈ 3.1622777 maps to slice 12 of 24.
        assert_eq!(
            cluster_depth_slice(10.0_f32.sqrt(), 0.1, 100.0, 24),
            Some(12)
        );
        assert!(cluster_depth_slice(1.0, 0.0, 100.0, 24).is_none());
    }
    #[test]
    fn oren_nayar_lunar_sphere_and_capsule_lights_match_analytic_properties() {
        let lambert = oren_nayar_diffuse_reflectance([0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0], 1.0, 0.0);
        assert!((lambert - std::f32::consts::FRAC_1_PI).abs() < 1e-5);
        let rough_retro = oren_nayar_diffuse_reflectance(
            [0.0, 0.0, 1.0],
            [0.707_106_77, 0.0, 0.707_106_77],
            [0.707_106_77, 0.0, 0.707_106_77],
            1.0,
            0.9,
        );
        let rough_forward = oren_nayar_diffuse_reflectance(
            [0.0, 0.0, 1.0],
            [0.707_106_77, 0.0, 0.707_106_77],
            [-0.707_106_77, 0.0, 0.707_106_77],
            1.0,
            0.9,
        );
        assert!(rough_retro > rough_forward);

        let opposition = lunar_dust_retroreflection(0.8, 0.8, 1.0, 1.0, 0.1);
        let oblique = lunar_dust_retroreflection(0.8, 0.8, 0.0, 1.0, 0.1);
        assert!(opposition > oblique);

        let mrp = sphere_light_mrt_representative_point([0.0, 0.0, 1.0], [1.0, 0.0, 5.0], 0.5);
        assert!((mrp[0] - 0.5).abs() < 1e-5 && (mrp[2] - 5.0).abs() < 1e-5);

        let cap = capsule_light_irradiance_factor(
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [-1.0, 0.0, 2.0],
            [1.0, 0.0, 2.0],
        );
        assert!(cap > 0.0);
        let below = capsule_light_irradiance_factor(
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [-1.0, 0.0, -2.0],
            [1.0, 0.0, -2.0],
        );
        assert_eq!(below, 0.0);
    }
}
