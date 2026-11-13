//! GPU instancing for `SceneRenderer`: draws `N` instances of a `SceneGeometry`
//! in a single `draw_indexed(0..index_count, 0, 0..instance_count)` command using
//! `@builtin(instance_index)` and a storage buffer of per-instance transforms and tints.

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use voxy_gpu_core::ManagedBufferDescriptor;

use super::{
    SceneDepthMode, SceneError, SceneGeometry, SceneRenderer, SceneTexture,
    resources::scene_memory_error,
};

const INSTANCED_SCENE_SHADER: &str = r"
struct InstanceData {
    mvp: mat4x4<f32>,
    tint: vec4<f32>,
}

@group(0) @binding(0) var<storage, read> instances: array<InstanceData>;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_instanced(
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
) -> VertexOutput {
    let inst = instances[instance_index];
    var out: VertexOutput;
    out.position = inst.mvp * vec4<f32>(position, 1.0);
    out.uv = uv;
    out.color = color * inst.tint;
    return out;
}

@fragment
fn fs_instanced_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sample = textureSample(image, image_sampler, in.uv) * in.color;
    if sample.a <= 0.0 { discard; }
    return sample;
}

@fragment
fn fs_instanced_solid_opaque(in: VertexOutput) -> @location(0) vec4<f32> {
    let sample = textureSample(image, image_sampler, in.uv) * in.color;
    return vec4<f32>(sample.rgb, 1.0);
}
";

/// Per-instance GPU payload (80 bytes: `mvp` matrix + RGBA `tint`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct SceneInstanceData {
    pub mvp: [[f32; 4]; 4],
    pub tint: [f32; 4],
}

impl SceneInstanceData {
    /// Creates an instance with the given `mvp` transform and white `[1.0; 4]` tint.
    #[must_use]
    pub fn new(mvp: Mat4) -> Self {
        Self {
            mvp: mvp.to_cols_array_2d(),
            tint: [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// Creates an instance with the given `mvp` transform and linear RGBA `tint`.
    #[must_use]
    pub fn with_tint(mvp: Mat4, tint: [f32; 4]) -> Self {
        Self {
            mvp: mvp.to_cols_array_2d(),
            tint,
        }
    }

    #[must_use]
    pub fn is_finite(&self) -> bool {
        self.mvp.iter().flatten().all(|v| v.is_finite()) && self.tint.iter().all(|v| v.is_finite())
    }
}

/// Retained GPU storage buffer and bind group holding up to `capacity` instance records.
#[derive(Debug)]
pub struct SceneInstanceBatch {
    device: wgpu::Device,
    buffer: Arc<crate::ComputeStorage>,
    bind_group: wgpu::BindGroup,
    count: u32,
    capacity: usize,
}

impl SceneInstanceBatch {
    #[must_use]
    pub const fn len(&self) -> u32 {
        self.count
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    #[must_use]
    pub fn allocation_bytes(&self) -> u64 {
        self.buffer.size()
    }

    /// Updates the active instance slice in-place without reallocating the GPU buffer.
    ///
    /// # Errors
    /// Rejects empty slices, slices exceeding `capacity`, or non-finite transforms/tints.
    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
        instances: &[SceneInstanceData],
    ) -> Result<(), SceneError> {
        if instances.is_empty() || instances.len() > self.capacity {
            return Err(SceneError::GeometryCapacityExceeded);
        }
        if instances.iter().any(|inst| !inst.is_finite()) {
            return Err(SceneError::InvalidTransform);
        }
        let count = u32::try_from(instances.len()).map_err(|_| SceneError::InvalidGeometry)?;
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(instances));
        self.count = count;
        Ok(())
    }
}

/// Borrowed instanced draw submission for `SceneInstancer`.
#[derive(Clone, Copy, Debug)]
pub struct SceneInstancedDraw<'a> {
    pub geometry: &'a SceneGeometry,
    pub texture: &'a SceneTexture,
    pub instances: &'a SceneInstanceBatch,
    pub overlay: bool,
}

/// GPU instancing pipeline set paired with a `SceneRenderer`.
#[derive(Debug)]
pub struct SceneInstancer {
    device: wgpu::Device,
    instance_layout: wgpu::BindGroupLayout,
    solid_opaque_pipeline: wgpu::RenderPipeline,
    world_pipeline: wgpu::RenderPipeline,
    overlay_pipeline: wgpu::RenderPipeline,
    transparent_pipeline: wgpu::RenderPipeline,
    reverse_z: bool,
}

impl SceneInstancer {
    /// Creates an instanced rendering pipeline set sharing the `SceneRenderer`'s device,
    /// attachment format, and Reversed-Z depth convention.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub fn new(scene: &SceneRenderer) -> Self {
        let device = &scene.device;
        let instance_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene instance batch layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<SceneInstanceData>() as u64,
                    ),
                },
                count: None,
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("instanced scene shader"),
            source: wgpu::ShaderSource::Wgsl(INSTANCED_SCENE_SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("instanced scene pipeline layout"),
            bind_group_layouts: &[Some(&instance_layout), Some(&scene.texture_layout)],
            immediate_size: 0,
        });
        let attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4];
        let normal_attributes = wgpu::vertex_attr_array![3 => Float32x3];
        let coordinate_attributes = wgpu::vertex_attr_array![5 => Float32x3];
        let vertex_buffers = [
            Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<super::SceneVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &attributes,
            }),
            Some(wgpu::VertexBufferLayout {
                array_stride: 12,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &normal_attributes,
            }),
            None,
            Some(wgpu::VertexBufferLayout {
                array_stride: 12,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &coordinate_attributes,
            }),
        ];
        let depth_compare = if scene.reverse_z {
            wgpu::CompareFunction::GreaterEqual
        } else {
            wgpu::CompareFunction::LessEqual
        };
        let build_pipeline = |label: &str,
                              fs_entry: &str,
                              cull_mode: Option<wgpu::Face>,
                              depth_write_enabled: bool,
                              depth_compare: wgpu::CompareFunction,
                              blend: wgpu::BlendState| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_instanced"),
                    compilation_options: Default::default(),
                    buffers: &vertex_buffers,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fs_entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: scene.color_format,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    cull_mode,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(depth_write_enabled),
                    depth_compare: Some(depth_compare),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let solid_opaque_pipeline = build_pipeline(
            "instanced solid opaque scene pipeline",
            "fs_instanced_solid_opaque",
            Some(wgpu::Face::Back),
            true,
            depth_compare,
            wgpu::BlendState::REPLACE,
        );
        let world_pipeline = build_pipeline(
            "instanced world scene pipeline",
            "fs_instanced_main",
            None,
            true,
            depth_compare,
            wgpu::BlendState::ALPHA_BLENDING,
        );
        let overlay_pipeline = build_pipeline(
            "instanced overlay scene pipeline",
            "fs_instanced_main",
            None,
            false,
            wgpu::CompareFunction::Always,
            wgpu::BlendState::ALPHA_BLENDING,
        );
        let transparent_pipeline = build_pipeline(
            "instanced transparent scene pipeline",
            "fs_instanced_main",
            None,
            false,
            depth_compare,
            wgpu::BlendState::ALPHA_BLENDING,
        );
        Self {
            device: device.clone(),
            instance_layout,
            solid_opaque_pipeline,
            world_pipeline,
            overlay_pipeline,
            transparent_pipeline,
            reverse_z: scene.reverse_z,
        }
    }

    /// Uploads an initial batch of instances into a managed GPU storage buffer.
    ///
    /// # Errors
    /// Rejects foreign devices, empty batches, non-finite transforms/tints, or memory budget exhaustion.
    pub fn create_batch(
        &self,
        device: &wgpu::Device,
        instances: &[SceneInstanceData],
    ) -> Result<SceneInstanceBatch, SceneError> {
        if &self.device != device {
            return Err(SceneError::DeviceMismatch);
        }
        if instances.is_empty() {
            return Err(SceneError::InvalidGeometry);
        }
        if instances.iter().any(|inst| !inst.is_finite()) {
            return Err(SceneError::InvalidTransform);
        }
        let count = u32::try_from(instances.len()).map_err(|_| SceneError::InvalidGeometry)?;
        let bytes = bytemuck::cast_slice(instances);
        let [buffer] = crate::ComputeMemoryBudget::for_device(device)
            .allocate_buffers([ManagedBufferDescriptor {
                label: "scene instance batch",
                size: bytes.len() as u64,
                contents: Some(bytes),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            }])
            .map_err(scene_memory_error)?;
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene instance batch bind group"),
            layout: &self.instance_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        Ok(SceneInstanceBatch {
            device: device.clone(),
            buffer: Arc::new(buffer),
            bind_group,
            count,
            capacity: instances.len(),
        })
    }

    /// Encodes instanced draws into an active render pass using state deduplication.
    pub fn encode_draws_in_pass(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        draws: &[SceneInstancedDraw<'_>],
    ) {
        let mut last_pipeline_ptr = 0usize;
        let mut last_geom_key: Option<(u64, u64)> = None;
        let mut last_tex_ptr = 0usize;

        for stage in 0..4u8 {
            for draw in draws.iter().filter(|draw| {
                if draw.instances.is_empty()
                    || draw.instances.device != self.device
                    || !draw.geometry.belongs_to(&self.device)
                {
                    return false;
                }
                let draw_stage = if draw.overlay {
                    3
                } else {
                    match draw.geometry.depth_mode {
                        SceneDepthMode::SolidOpaque | SceneDepthMode::Opaque => 0,
                        SceneDepthMode::Transparent => 1,
                        SceneDepthMode::Xray => 2,
                    }
                };
                draw_stage == stage
            }) {
                let pipeline = match stage {
                    0 if draw.geometry.depth_mode == SceneDepthMode::SolidOpaque => {
                        &self.solid_opaque_pipeline
                    }
                    0 => &self.world_pipeline,
                    1 => &self.transparent_pipeline,
                    _ => &self.overlay_pipeline,
                };
                let pipeline_ptr = std::ptr::from_ref(pipeline) as usize;
                if last_pipeline_ptr != pipeline_ptr {
                    pass.set_pipeline(pipeline);
                    last_pipeline_ptr = pipeline_ptr;
                }
                pass.set_bind_group(0, &draw.instances.bind_group, &[]);
                let tex_ptr = std::ptr::from_ref(draw.texture) as usize;
                if last_tex_ptr != tex_ptr {
                    pass.set_bind_group(1, &draw.texture.bind_group, &[]);
                    last_tex_ptr = tex_ptr;
                }
                let geom_key = draw.geometry.shadow_key();
                if last_geom_key != Some(geom_key) {
                    draw.geometry.bind_instanced_streams(pass);
                    last_geom_key = Some(geom_key);
                }
                pass.draw_indexed(0..draw.geometry.index_count, 0, 0..draw.instances.count);
            }
        }
    }

    /// Encodes instanced draws over existing color and depth attachments without clearing them.
    pub fn encode_over(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        draws: &[SceneInstancedDraw<'_>],
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("instanced scene over existing color and depth"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        self.encode_draws_in_pass(&mut pass, draws);
    }

    /// Clears color and depth attachments and encodes instanced draws.
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        clear: wgpu::Color,
        draws: &[SceneInstancedDraw<'_>],
    ) {
        let depth_clear = if self.reverse_z { 0.0 } else { 1.0 };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("instanced scene pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(depth_clear),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        self.encode_draws_in_pass(&mut pass, draws);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{SceneMesh, SceneVertex};

    #[test]
    fn instanced_scene_shader_validates_and_batches_instances() {
        let module = naga::front::wgsl::parse_str(INSTANCED_SCENE_SHADER).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();

        let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let scene = SceneRenderer::new_reverse_z(&device, wgpu::TextureFormat::Rgba16Float);
        let instancer = SceneInstancer::new(&scene);
        let mesh = SceneMesh::new(
            vec![
                SceneVertex {
                    position: [0.0, 0.0, 0.5],
                    uv: [0.0, 0.0],
                    color: [1.0; 4],
                },
                SceneVertex {
                    position: [0.5, 0.0, 0.5],
                    uv: [1.0, 0.0],
                    color: [1.0; 4],
                },
                SceneVertex {
                    position: [0.0, 0.5, 0.5],
                    uv: [0.0, 1.0],
                    color: [1.0; 4],
                },
            ],
            vec![0, 1, 2],
        )
        .unwrap();
        let geometry = scene.upload_mesh(&device, &mesh).unwrap();
        let texture = scene
            .upload_texture(&device, &queue, 1, 1, &[255, 255, 255, 255])
            .unwrap();
        let instances = [
            SceneInstanceData::new(Mat4::IDENTITY),
            SceneInstanceData::with_tint(
                Mat4::from_translation(glam::Vec3::new(2.0, 0.0, 0.0)),
                [0.8, 0.9, 1.0, 1.0],
            ),
            SceneInstanceData::new(Mat4::from_translation(glam::Vec3::new(-2.0, 0.0, 0.0))),
        ];
        let mut batch = instancer.create_batch(&device, &instances).unwrap();
        assert_eq!(batch.len(), 3);
        assert_eq!(batch.capacity(), 3);
        batch.update(&queue, &instances[..2]).unwrap();
        assert_eq!(batch.len(), 2);

        let color = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("instanced color"),
                size: wgpu::Extent3d {
                    width: 64,
                    height: 64,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        let depth = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("instanced depth"),
                size: wgpu::Extent3d {
                    width: 64,
                    height: 64,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        instancer.encode(
            &mut encoder,
            &color,
            &depth,
            wgpu::Color::BLACK,
            &[SceneInstancedDraw {
                geometry: &geometry,
                texture: &texture,
                instances: &batch,
                overlay: false,
            }],
        );
        queue.submit([encoder.finish()]);
    }
}
