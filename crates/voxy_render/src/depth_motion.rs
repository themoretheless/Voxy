//! Static-world camera motion covering primary opaque depth.
use crate::RaySceneError;
use glam::Mat4;
use wgpu::util::DeviceExt;
#[derive(Debug)]
pub struct DepthMotionPass {
    pipeline: wgpu::RenderPipeline,
    bindings: wgpu::BindGroup,
    output: wgpu::Texture,
}
impl DepthMotionPass {
    /// Reconstruct static world positions from primary depth and project through the previous camera.
    /// Both cameras must be unjittered, and current must exactly match primary depth.
    /// Use `clear_depth=0` for reverse-Z or 1 for conventional depth. Moving geometry
    /// must overwrite these vectors using matching object/deformation correspondence.
    /// # Errors
    /// Rejects invalid cameras, clear depth or incompatible primary depth resources.
    #[allow(clippy::too_many_lines)]
    pub fn new(
        device: &wgpu::Device,
        depth: &wgpu::Texture,
        cameras: [Mat4; 2],
        clear_depth: f32,
        reset: bool,
    ) -> Result<Self, RaySceneError> {
        Self::with_jittered_depth(device, depth, cameras[0], cameras, clear_depth, reset)
    }
    /// Reconstruct with the exact jittered depth camera, but emit unjittered camera motion.
    /// This separates raster jitter from actual camera motion; moving geometry still
    /// needs an object/deformation overwrite at the same primary coverage.
    /// # Errors
    /// Rejects invalid depth or camera matrices and the same resources as `new`.
    #[allow(clippy::too_many_lines)]
    pub fn with_jittered_depth(
        device: &wgpu::Device,
        depth: &wgpu::Texture,
        depth_camera: Mat4,
        cameras: [Mat4; 2],
        clear_depth: f32,
        reset: bool,
    ) -> Result<Self, RaySceneError> {
        for matrix in [depth_camera, cameras[0], cameras[1]] {
            let determinant = matrix.determinant();
            if !matrix.is_finite() || !determinant.is_finite() || determinant == 0.0 {
                return Err(RaySceneError::InvalidGeometry);
            }
        }
        let inverse = depth_camera.inverse();
        if !inverse.is_finite()
            || !clear_depth.is_finite()
            || !(0.0..=1.0).contains(&clear_depth)
            || depth.format() != wgpu::TextureFormat::Depth32Float
            || depth.dimension() != wgpu::TextureDimension::D2
            || depth.depth_or_array_layers() != 1
            || depth.sample_count() != 1
            || !depth.usage().contains(wgpu::TextureUsages::TEXTURE_BINDING)
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        if device.limits().max_sampled_textures_per_shader_stage < 1
            || device.limits().max_color_attachment_bytes_per_sample < 4
        {
            return Err(RaySceneError::Capacity);
        }
        let mut data = bytemuck::cast_slice(&inverse.to_cols_array()).to_vec();
        data.extend_from_slice(bytemuck::cast_slice(&cameras[0].to_cols_array()));
        data.extend_from_slice(bytemuck::cast_slice(&cameras[1].to_cols_array()));
        data.extend_from_slice(bytemuck::cast_slice(&[
            clear_depth,
            if reset { 1.0 } else { 0.0 },
            0.0,
            0.0,
        ]));
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("static depth motion cameras"),
            contents: &data,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("static depth RG16 motion"),
            size: depth.size(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rg16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("static depth motion"),
            source: wgpu::ShaderSource::Wgsl(crate::depth_sample::shader(
                device,
                include_str!("depth_motion.wgsl"),
            )),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("static depth motion"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: output.format(),
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let view = depth.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_sampler = crate::depth_sample::sampler(device);
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("static depth motion"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&depth_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
            ],
        });
        Ok(Self {
            pipeline,
            bindings,
            output,
        })
    }
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let view = self
            .output
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("static depth motion"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bindings, &[]);
        pass.draw(0..3, 0..1);
    }
    #[must_use]
    pub fn output(&self) -> &wgpu::Texture {
        &self.output
    }
}
