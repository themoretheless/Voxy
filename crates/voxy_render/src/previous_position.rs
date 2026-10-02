//! Rasterize previous world positions at current-frame opaque coverage.
use crate::RaySceneError;
use glam::Mat4;
use wgpu::util::DeviceExt;

/// Paired triangle-list vertices from matching current/previous animated poses.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PreviousPositionVertex {
    pub current: [f32; 3],
    pub previous: [f32; 3],
}
#[derive(Debug)]
pub struct PreviousPositionPass {
    device: wgpu::Device,
    pipeline: wgpu::RenderPipeline,
    bindings: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    count: u32,
    depth: wgpu::TextureView,
    output: wgpu::Texture,
    position_origin: [f32; 3],
}
impl PreviousPositionPass {
    /// Rasterize paired world-space vertices with the exact current primary camera.
    /// Depth must come from the matching opaque primary pass. Equal-depth testing
    /// preserves occlusion without writing depth. Background clears to invalid W=0.
    /// Topology and vertex correspondence are the caller's responsibility.
    /// OpenGL uses an `RGBA16Float` position target; inspect `output().format()`
    /// and account for half-float world-position precision. Other backends use RGBA32.
    /// OpenGL rejects previous coordinates outside the finite half-float range.
    /// # Errors
    /// Rejects invalid vertices, matrices, depth resources or capacity.
    #[allow(clippy::too_many_lines)]
    pub fn new(
        device: &wgpu::Device,
        depth: &wgpu::Texture,
        camera: Mat4,
        vertices: &[PreviousPositionVertex],
    ) -> Result<Self, RaySceneError> {
        Self::create(device, depth, camera, vertices, None, None, false)
    }
    /// Store previous positions relative to a finite origin to retain local half-float precision.
    /// Current positions/camera remain world-space. Pass `position_origin()` to the motion consumer.
    /// # Errors
    /// Rejects invalid origin, shifted coordinates or the same inputs as `new`.
    pub fn new_relative(
        device: &wgpu::Device,
        depth: &wgpu::Texture,
        camera: Mat4,
        vertices: &[PreviousPositionVertex],
        origin: [f32; 3],
    ) -> Result<Self, RaySceneError> {
        let offset = glam::Vec3::from_array(origin);
        if !offset.is_finite() {
            return Err(RaySceneError::InvalidGeometry);
        }
        let shifted = vertices
            .iter()
            .map(|vertex| PreviousPositionVertex {
                current: vertex.current,
                previous: (glam::Vec3::from_array(vertex.previous) - offset).to_array(),
            })
            .collect::<Vec<_>>();
        let mut pass = Self::new(device, depth, camera, &shifted)?;
        pass.position_origin = origin;
        Ok(pass)
    }
    #[must_use]
    pub const fn position_origin(&self) -> [f32; 3] {
        self.position_origin
    }
    #[allow(clippy::too_many_lines)]
    fn create(
        device: &wgpu::Device,
        depth: &wgpu::Texture,
        camera: Mat4,
        vertices: &[PreviousPositionVertex],
        motion: Option<(Mat4, bool)>,
        cached_pipeline: Option<(&wgpu::RenderPipeline, Option<&wgpu::Texture>)>,
        previous_depth: bool,
    ) -> Result<Self, RaySceneError> {
        if let Some((previous, _)) = motion {
            let determinant = previous.determinant();
            if !previous.is_finite() || !determinant.is_finite() || determinant == 0.0 {
                return Err(RaySceneError::InvalidGeometry);
            }
        }
        let half_positions = motion.is_none() && device.adapter_info().backend == wgpu::Backend::Gl;
        if half_positions
            && vertices
                .iter()
                .any(|vertex| vertex.previous.iter().any(|value| value.abs() > 65504.0))
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        let count = u32::try_from(vertices.len()).map_err(|_| RaySceneError::Capacity)?;
        let determinant = camera.determinant();
        if count == 0
            || !count.is_multiple_of(3)
            || vertices
                .iter()
                .any(|v| v.current.iter().chain(&v.previous).any(|x| !x.is_finite()))
            || !camera.is_finite()
            || !determinant.is_finite()
            || determinant == 0.0
            || depth.format() != wgpu::TextureFormat::Depth32Float
            || depth.dimension() != wgpu::TextureDimension::D2
            || depth.depth_or_array_layers() != 1
            || depth.sample_count() != 1
            || !depth
                .usage()
                .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        let bytes = bytemuck::cast_slice(vertices);
        if u64::try_from(bytes.len()).map_err(|_| RaySceneError::Capacity)?
            > device.limits().max_buffer_size
            || device.limits().max_color_attachment_bytes_per_sample
                < if motion.is_some() { 4 } else { 16 }
        {
            return Err(RaySceneError::Capacity);
        }
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("current/previous pose vertices"),
            contents: bytes,
            usage: wgpu::BufferUsages::VERTEX,
        });
        let mut camera_data = bytemuck::cast_slice(&camera.to_cols_array()).to_vec();
        if let Some((previous, reset)) = motion {
            camera_data.extend_from_slice(bytemuck::cast_slice(&previous.to_cols_array()));
            camera_data.extend_from_slice(bytemuck::cast_slice(&[u32::from(reset), 0, 0, 0]));
        }
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("correspondence camera"),
            contents: &camera_data,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let output = cached_pipeline
            .and_then(|(_, output)| output)
            .filter(|output| output.size() == depth.size())
            .cloned()
            .unwrap_or_else(|| {
                device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("previous position correspondence"),
                    size: depth.size(),
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: if previous_depth {
                        if device.adapter_info().backend == wgpu::Backend::Gl {
                            wgpu::TextureFormat::R16Float
                        } else {
                            wgpu::TextureFormat::R32Float
                        }
                    } else if motion.is_some() {
                        wgpu::TextureFormat::Rg16Float
                    } else if half_positions {
                        wgpu::TextureFormat::Rgba16Float
                    } else {
                        wgpu::TextureFormat::Rgba32Float
                    },
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                })
            });
        let pipeline = if let Some((pipeline, _)) = cached_pipeline {
            pipeline.clone()
        } else {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("previous position raster"),
                source: wgpu::ShaderSource::Wgsl(
                    if previous_depth {
                        include_str!("previous_depth.wgsl")
                    } else if motion.is_some() {
                        include_str!("raster_motion.wgsl")
                    } else {
                        include_str!("previous_position.wgsl")
                    }
                    .into(),
                ),
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("previous position raster"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: 24,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                    })],
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
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: depth.format(),
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::Equal),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("correspondence camera"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        Ok(Self {
            device: device.clone(),
            pipeline,
            bindings,
            vertices,
            count,
            depth: depth.create_view(&wgpu::TextureViewDescriptor::default()),
            output,
            position_origin: [0.0; 3],
        })
    }
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let view = self
            .output
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.encode_target(
            encoder,
            &view,
            wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
        );
    }
    fn encode_target(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        load: wgpu::LoadOp<wgpu::Color>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("previous pose correspondence"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bindings, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw(0..self.count, 0..1);
    }
    #[must_use]
    pub fn output(&self) -> &wgpu::Texture {
        &self.output
    }
}

/// Depth-tested backward UV motion for matched current/previous world geometry.
#[derive(Debug)]
pub struct RasterMotionPass(PreviousPositionPass);
impl RasterMotionPass {
    /// Cameras are unjittered and must match primary coverage; geometry is paired by topology.
    /// Output is `RG16Float`, top-left backward UV, with zero background/reset motion.
    /// # Errors
    /// Rejects invalid cameras, paired vertices, depth resources or device capacity.
    pub fn new(
        device: &wgpu::Device,
        depth: &wgpu::Texture,
        cameras: [Mat4; 2],
        vertices: &[PreviousPositionVertex],
        reset: bool,
    ) -> Result<Self, RaySceneError> {
        PreviousPositionPass::create(
            device,
            depth,
            cameras[0],
            vertices,
            Some((cameras[1], reset)),
            None,
            false,
        )
        .map(Self)
    }
    /// Create independent frame resources while sharing the compiled motion pipeline.
    /// All resources and the supplied device must belong to the same device as `self`.
    /// # Errors
    /// Applies the same geometry/camera/depth validation as `new`.
    pub fn next_frame(
        &self,
        device: &wgpu::Device,
        depth: &wgpu::Texture,
        cameras: [Mat4; 2],
        vertices: &[PreviousPositionVertex],
        reset: bool,
    ) -> Result<Self, RaySceneError> {
        if device != &self.0.device {
            return Err(RaySceneError::DeviceMismatch);
        }
        PreviousPositionPass::create(
            device,
            depth,
            cameras[0],
            vertices,
            Some((cameras[1], reset)),
            Some((&self.0.pipeline, None)),
            false,
        )
        .map(Self)
    }
    /// Transfer motion output storage and reuse the compiled pipeline.
    /// Order old consumers before this pass on the same queue; resize replaces output.
    /// # Errors
    /// Rejects foreign devices and preserves normal input validation errors.
    pub fn next_frame_reusing(
        self,
        device: &wgpu::Device,
        depth: &wgpu::Texture,
        cameras: [Mat4; 2],
        vertices: &[PreviousPositionVertex],
        reset: bool,
    ) -> Result<Self, RaySceneError> {
        if device != &self.0.device {
            return Err(RaySceneError::DeviceMismatch);
        }
        PreviousPositionPass::create(
            device,
            depth,
            cameras[0],
            vertices,
            Some((cameras[1], reset)),
            Some((&self.0.pipeline, Some(&self.0.output))),
            false,
        )
        .map(Self)
    }
    /// Overwrite only this geometry's visible coverage, preserving existing RG16 motion elsewhere.
    /// # Errors
    /// Rejects incompatible target dimensions, format, samples or attachment usage.
    pub fn encode_over(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::Texture,
    ) -> Result<(), RaySceneError> {
        if target.size() != self.output().size()
            || target.format() != wgpu::TextureFormat::Rg16Float
            || target.sample_count() != 1
            || target.dimension() != wgpu::TextureDimension::D2
            || !target
                .usage()
                .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        self.0.encode_target(encoder, &view, wgpu::LoadOp::Load);
        Ok(())
    }
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        self.0.encode(encoder);
    }
    #[must_use]
    pub fn output(&self) -> &wgpu::Texture {
        self.0.output()
    }
}

/// Previous-camera NDC depth at current-frame opaque coverage.
#[derive(Debug)]
pub struct PreviousDepthPass(PreviousPositionPass);
impl PreviousDepthPass {
    /// Geometry/cameras must match the opaque primary pass. Paired previous
    /// vertices follow deformation; background/behind-camera/out-of-range is zero.
    /// Encode after primary coverage. Output is sampleable R32Float for rejection,
    /// or renderable R16Float with half precision on OpenGL.
    /// # Errors
    /// Rejects invalid paired geometry, matrices, depth resources or capacity.
    pub fn new(
        device: &wgpu::Device,
        depth: &wgpu::Texture,
        cameras: [Mat4; 2],
        vertices: &[PreviousPositionVertex],
    ) -> Result<Self, RaySceneError> {
        PreviousPositionPass::create(
            device,
            depth,
            cameras[0],
            vertices,
            Some((cameras[1], false)),
            None,
            true,
        )
        .map(Self)
    }
    /// Prepare independent output while sharing the compiled depth pipeline.
    /// # Errors
    /// Rejects foreign devices and preserves input validation errors.
    pub fn next_frame(
        &self,
        device: &wgpu::Device,
        depth: &wgpu::Texture,
        cameras: [Mat4; 2],
        vertices: &[PreviousPositionVertex],
    ) -> Result<Self, RaySceneError> {
        if device != &self.0.device {
            return Err(RaySceneError::DeviceMismatch);
        }
        PreviousPositionPass::create(
            device,
            depth,
            cameras[0],
            vertices,
            Some((cameras[1], false)),
            Some((&self.0.pipeline, None)),
            true,
        )
        .map(Self)
    }
    /// Transfer depth output storage for an explicitly ordered later pass.
    /// Do not transfer a texture while temporal history still needs its old contents.
    /// # Errors
    /// Rejects foreign devices and preserves input validation errors.
    pub fn next_frame_reusing(
        self,
        device: &wgpu::Device,
        depth: &wgpu::Texture,
        cameras: [Mat4; 2],
        vertices: &[PreviousPositionVertex],
    ) -> Result<Self, RaySceneError> {
        if device != &self.0.device {
            return Err(RaySceneError::DeviceMismatch);
        }
        PreviousPositionPass::create(
            device,
            depth,
            cameras[0],
            vertices,
            Some((cameras[1], false)),
            Some((&self.0.pipeline, Some(&self.0.output))),
            true,
        )
        .map(Self)
    }
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        self.0.encode(encoder);
    }
    #[must_use]
    pub fn output(&self) -> &wgpu::Texture {
        self.0.output()
    }
}
