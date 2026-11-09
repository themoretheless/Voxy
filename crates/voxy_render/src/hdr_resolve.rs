//! Explicit wide HDR to linear half-float conversion before SDK handoff.
use crate::{RaySceneError, TextureBlit};
use std::sync::Arc;

/// Reusable unexposed linear HDR resolve. No transfer function or tone mapping.
#[derive(Debug)]
pub struct HdrHalfResolvePipeline {
    device: wgpu::Device,
    blit: Arc<TextureBlit>,
}
impl HdrHalfResolvePipeline {
    /// # Errors
    /// Rejects insufficient sampling/uniform limits or unsupported output configuration.
    pub fn new(device: &wgpu::Device) -> Result<Self, RaySceneError> {
        let limits = device.limits();
        if limits.max_sampled_textures_per_shader_stage < 1
            || limits.max_uniform_buffers_per_shader_stage < 2
            || limits.max_uniform_buffer_binding_size < 16
            || limits.max_bindings_per_bind_group < 3
        {
            return Err(RaySceneError::Capacity);
        }
        let blit = TextureBlit::linear_exposed(device, wgpu::TextureFormat::Rgba16Float, 1.0)
            .ok_or(RaySceneError::Unsupported)?;
        Ok(Self {
            device: device.clone(),
            blit: Arc::new(blit),
        })
    }
    /// Allocate independent sampleable half-float output at the source resolution.
    /// Source must share this device and contain finite linear RGB and alpha in [0,1].
    /// RGB clips to [0,65504]; alpha is preserved. Only mip level zero is resolved.
    /// Encode after source production.
    /// # Errors
    /// Rejects nonfloat, layered, multisampled or unsampleable source textures.
    pub fn prepare(&self, source: &wgpu::Texture) -> Result<HdrHalfResolveJob, RaySceneError> {
        if !matches!(
            source.format(),
            wgpu::TextureFormat::Rgba16Float | wgpu::TextureFormat::Rgba32Float
        ) || source.dimension() != wgpu::TextureDimension::D2
            || source.depth_or_array_layers() != 1
            || source.sample_count() != 1
            || !source
                .usage()
                .contains(wgpu::TextureUsages::TEXTURE_BINDING)
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        let output = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("linear HDR half-float SDK input"),
            size: source.size(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        Ok(HdrHalfResolveJob {
            device: self.device.clone(),
            blit: self.blit.clone(),
            source: source.create_view(&wgpu::TextureViewDescriptor {
                mip_level_count: Some(1),
                ..Default::default()
            }),
            output,
        })
    }
}
#[derive(Debug)]
pub struct HdrHalfResolveJob {
    device: wgpu::Device,
    blit: Arc<TextureBlit>,
    source: wgpu::TextureView,
    output: wgpu::Texture,
}
impl HdrHalfResolveJob {
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        self.blit.encode(
            &self.device,
            encoder,
            &self.source,
            &self
                .output
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
    }
    #[must_use]
    pub fn output(&self) -> &wgpu::Texture {
        &self.output
    }
}

/// Evaluates an edge-preserving bilateral upsampling weight in `[0.0, 1.0]` between a full-res
/// target pixel `(hi_depth, hi_normal)` and a half-res sample `(lo_depth, lo_normal)` for
/// half-resolution Screen-Space GI and volumetric shafts.
#[must_use]
pub fn bilateral_upsample_weight(
    hi_depth: f32,
    lo_depth: f32,
    hi_normal: [f32; 3],
    lo_normal: [f32; 3],
    depth_sigma: f32,
    normal_power: f32,
) -> f32 {
    if !hi_depth.is_finite()
        || !lo_depth.is_finite()
        || !depth_sigma.is_finite()
        || !normal_power.is_finite()
        || hi_depth <= 0.0
        || lo_depth <= 0.0
        || depth_sigma <= 0.0
        || normal_power < 0.0
    {
        return 0.0;
    }
    let rel_depth = (hi_depth - lo_depth).abs() / hi_depth.max(1e-4);
    let w_depth = (-0.5 * (rel_depth / depth_sigma).powi(2)).exp();
    let nh = glam::Vec3::from_array(hi_normal).normalize_or_zero();
    let nl = glam::Vec3::from_array(lo_normal).normalize_or_zero();
    let dot = nh.dot(nl).clamp(0.0, 1.0);
    let w_norm = dot.powf(normal_power);
    (w_depth * w_norm).clamp(0.0, 1.0)
}

/// Evaluates the Khronos PBR Neutral tone mapper on linear RGB, preserving exact 1:1 base color
/// reproduction up to `start_compression = 0.76` before smoothly compressing highlights with
/// controlled desaturation (`d = 0.15`).
#[must_use]
pub fn pbr_neutral_tonemap_rgb(linear_rgb: [f32; 3]) -> [f32; 3] {
    if linear_rgb.iter().any(|c| !c.is_finite()) {
        return [0.0; 3];
    }
    let mut color = linear_rgb.map(|c| c.max(0.0));
    let start_compression = 0.8 - 0.04;
    let desaturation = 0.15;
    let x = color[0].min(color[1]).min(color[2]);
    let offset = if x < 0.08 {
        x - 6.25 * x * x
    } else {
        0.04
    };
    for c in &mut color {
        *c -= offset;
    }
    let peak = color[0].max(color[1]).max(color[2]);
    if peak < start_compression {
        return color.map(|c| c.clamp(0.0, 1.0));
    }
    let d = 1.0 - start_compression;
    let new_peak = 1.0 - d * d / (peak + d - start_compression);
    let scale = new_peak / peak.max(1e-6);
    for c in &mut color {
        *c *= scale;
    }
    let g = 1.0 - 1.0 / (desaturation * (peak - new_peak) + 1.0);
    color.map(|c| (c + g * (new_peak - c)).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bilateral_upsample_weight_rejects_discontinuous_depth_and_normal_edges() {
        let same = bilateral_upsample_weight(
            5.0,
            5.0,
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            0.05,
            16.0,
        );
        let edge = bilateral_upsample_weight(
            5.0,
            12.0,
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0],
            0.05,
            16.0,
        );
        assert!((same - 1.0).abs() < 1e-6);
        assert!(edge < 1e-4);

        let mid = pbr_neutral_tonemap_rgb([0.5, 0.25, 0.1]);
        assert!((mid[0] - 0.46).abs() < 1e-5);
        let hi = pbr_neutral_tonemap_rgb([10.0, 4.0, 1.0]);
        assert!(hi.iter().all(|&c| c > 0.0 && c <= 1.0));
    }
}
