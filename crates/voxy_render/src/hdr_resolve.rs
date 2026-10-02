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
