//! Spatial coverage for temporal AA, including frames with invalid history.
use crate::{ComputeError, ComputeMemoryBudget, ComputeStorage};
use std::sync::Arc;
#[derive(Debug)]
pub struct SupersampleResolve {
    pub(crate) device: wgpu::Device,
    pub(crate) color: Arc<voxy_gpu_core::ManagedTexture>,
    pub(crate) view: wgpu::TextureView,
    settings: ComputeStorage,
    pipeline: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
    width: u32,
    height: u32,
    scale: u32,
    format: wgpu::TextureFormat,
}
impl SupersampleResolve {
    /// Scale is per dimension: 2 renders four spatial samples per output pixel.
    /// The scene renderer must use `render_size()` and output linear HDR.
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        scale: u32,
        format: wgpu::TextureFormat,
    ) -> Result<Self, ComputeError> {
        if width == 0 || height == 0 || !(1..=4).contains(&scale) {
            return Err(ComputeError::InvalidDispatch);
        }
        let w = width
            .checked_mul(scale)
            .ok_or(ComputeError::InvalidDispatch)?;
        let h = height
            .checked_mul(scale)
            .ok_or(ComputeError::InvalidDispatch)?;
        if w > device.limits().max_texture_dimension_2d
            || h > device.limits().max_texture_dimension_2d
        {
            return Err(ComputeError::InvalidDispatch);
        }
        let color =
            ComputeMemoryBudget::for_device(device).allocate_texture(&wgpu::TextureDescriptor {
                label: Some("linear HDR spatial AA samples"),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            })?;
        let [settings] = ComputeMemoryBudget::for_device(device).allocate_buffers([
            voxy_gpu_core::ManagedBufferDescriptor {
                label: "spatial AA display settings",
                size: 16,
                contents: None,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            },
        ])?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("linear HDR area resolve"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("display_transform.wgsl"),
                    include_str!("supersampling.wgsl")
                )
                .into(),
            ),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("spatial AA area resolve"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let view = color.create_view(&Default::default());
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("spatial AA samples"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: settings.as_entire_binding(),
                },
            ],
        });
        Ok(Self {
            device: device.clone(),
            color,
            view,
            settings,
            pipeline,
            bind,
            width,
            height,
            scale,
            format,
        })
    }
    pub fn render_size(&self) -> (u32, u32) {
        (self.color.width(), self.color.height())
    }
    pub(crate) fn validate(&self, output: &wgpu::TextureView) -> Result<(), ComputeError> {
        if output.texture().width() != self.width
            || output.texture().height() != self.height
            || output.texture().sample_count() != 1
            || output.texture().format() != self.format
        {
            return Err(ComputeError::InvalidBuffer);
        }
        Ok(())
    }
    pub(crate) fn encode(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        mode: f32,
        headroom: f32,
    ) {
        queue.write_buffer(
            &self.settings,
            0,
            bytemuck::cast_slice(&[self.scale as f32, mode, headroom, 0.]),
        );
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("spatial AA final display"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// Computes an HDR-safe Robust Contrast-Adaptive Sharpening (RCAS) tap weight in `[-0.25, 0.0]`
/// from the 5-tap cross neighborhood luminances `(center, north, south, east, west)` and user
/// `sharpness` in `[0.0, 1.0]`, attenuating sharpening around high-contrast HDR highlights to
/// prevent negative ringing halos.
#[must_use]
pub fn rcas_sharpening_gain(center_luma: f32, neighbors_luma: [f32; 4], sharpness: f32) -> f32 {
    if !center_luma.is_finite()
        || neighbors_luma.iter().any(|v| !v.is_finite() || *v < 0.0)
        || !sharpness.is_finite()
        || center_luma < 0.0
    {
        return 0.0;
    }
    let s = sharpness.clamp(0.0, 1.0);
    if s <= 1e-6 {
        return 0.0;
    }
    let min_l = neighbors_luma.iter().copied().fold(center_luma, f32::min);
    let max_l = neighbors_luma
        .iter()
        .copied()
        .fold(center_luma, f32::max)
        .max(1e-5);
    let contrast = (max_l - min_l) / max_l;
    // Attenuate sharpening on high local contrast and HDR specular peaks (> 1.0):
    let hdr_damp = 1.0 / (1.0 + max_l.max(1.0) - 1.0);
    let adaptive = (1.0 - contrast).clamp(0.0, 1.0) * hdr_damp;
    (-0.25 * s * adaptive).clamp(-0.25, 0.0)
}

#[cfg(test)]
mod rcas_tests {
    use super::*;

    #[test]
    fn rcas_sharpening_gain_damps_hdr_highlights_and_zero_sharpness() {
        assert_eq!(rcas_sharpening_gain(0.5, [0.4, 0.5, 0.6, 0.5], 0.0), 0.0);
        let sdr = rcas_sharpening_gain(0.5, [0.45, 0.55, 0.48, 0.52], 1.0);
        let hdr = rcas_sharpening_gain(5.0, [4.5, 5.5, 4.8, 5.2], 1.0);
        assert!(sdr < 0.0 && sdr >= -0.25);
        assert!(hdr.abs() < sdr.abs());
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    #[test]
    fn metal_resolve_preserves_diagonal_coverage_and_linear_hdr() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let resolve =
            SupersampleResolve::new(&device, 4, 4, 2, wgpu::TextureFormat::Rgba16Float).unwrap();
        assert!(
            SupersampleResolve::new(&device, 4, 4, 0, wgpu::TextureFormat::Rgba16Float).is_err()
        );
        let mut source = Vec::new();
        for y in 0..8 {
            for x in 0..8 {
                let v = if x > y { 4. } else { 0. };
                source.extend([v, v, v, 1.].map(|v| half::f16::from_f32(v).to_bits()));
            }
        }
        queue.write_texture(
            resolve.color.as_image_copy(),
            bytemuck::cast_slice(&source),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(64),
                rows_per_image: Some(8),
            },
            wgpu::Extent3d {
                width: 8,
                height: 8,
                depth_or_array_layers: 1,
            },
        );
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 1024,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        for mode in [3., 0.] {
            let mut encoder = device.create_command_encoder(&Default::default());
            resolve.encode(
                &queue,
                &mut encoder,
                &output.create_view(&Default::default()),
                mode,
                2.,
            );
            encoder.copy_texture_to_buffer(
                output.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &read,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(4),
                    },
                },
                output.size(),
            );
            queue.submit([encoder.finish()]);
            let (tx, rx) = std::sync::mpsc::channel();
            read.map_async(wgpu::MapMode::Read, .., move |r| tx.send(r).unwrap());
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            rx.recv().unwrap().unwrap();
            let bytes = read.get_mapped_range(..).unwrap();
            for y in 0..4 {
                for x in 0..4 {
                    let offset = y * 256 + x * 8;
                    let value = half::f16::from_bits(u16::from_le_bytes([
                        bytes[offset],
                        bytes[offset + 1],
                    ]))
                    .to_f32();
                    let linear: f32 = match x.cmp(&y) {
                        std::cmp::Ordering::Greater => 4.,
                        std::cmp::Ordering::Equal => 1.,
                        std::cmp::Ordering::Less => 0.,
                    };
                    let expected = if mode == 3. {
                        linear
                    } else {
                        ((linear * (2.51 * linear + 0.03))
                            / (linear * (2.43 * linear + 0.59) + 0.14))
                            .clamp(0., 1.)
                    };
                    assert!(
                        (value - expected).abs() < 0.002,
                        "{x},{y}: {value} vs {expected}"
                    );
                }
            }
            drop(bytes);
            read.unmap();
        }
        assert!(pollster::block_on(scope.pop()).is_none());
    }
}
