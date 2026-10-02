//! Opaque world-space material guide rasterization for Ray Reconstruction.
use crate::{RayReconstructionGuides, ReconstructionMaterialError};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ReconstructionGuideVertex {
    position: [f32; 3],
    normal: [f32; 3],
    base_color: [f32; 3],
    metallic_roughness: [f32; 2],
    uv: [f32; 2],
}
impl ReconstructionGuideVertex {
    /// Supply world-space geometry and linear material values.
    /// # Errors
    /// Rejects invalid positions, degenerate normals or invalid material values.
    pub fn new(
        position: [f32; 3],
        normal: [f32; 3],
        base_color: [f32; 3],
        metallic: f32,
        roughness: f32,
    ) -> Result<Self, ReconstructionMaterialError> {
        crate::ReconstructionMaterial::new(base_color, metallic, roughness)?;
        if !glam::Vec3::from_array(position).is_finite() {
            return Err(ReconstructionMaterialError);
        }
        let normal = glam::Vec3::from_array(normal)
            .try_normalize()
            .ok_or(ReconstructionMaterialError)?;
        Ok(Self {
            position,
            normal: normal.to_array(),
            base_color,
            metallic_roughness: [metallic, roughness],
            uv: [0.0; 2],
        })
    }
    /// Set material UV coordinates; their sampler/wrapping convention belongs to the material.
    /// # Errors
    /// Rejects nonfinite UV values.
    pub fn with_uv(mut self, uv: [f32; 2]) -> Result<Self, ReconstructionMaterialError> {
        if uv.iter().any(|value| !value.is_finite()) {
            return Err(ReconstructionMaterialError);
        }
        self.uv = uv;
        Ok(self)
    }
}
#[derive(Debug)]
pub struct ReconstructionGuideMesh {
    vertices: wgpu::Buffer,
    count: u32,
}
impl ReconstructionGuideMesh {
    /// Upload indexed scene geometry using the scene renderer's smooth normals.
    /// Bakes an affine model transform into world positions and inverse-transpose
    /// normals. Vertex RGB is linear base color; textures and alpha are not sampled.
    /// Material parameters are uniform metallic/roughness for this mesh.
    /// # Errors
    /// Rejects invalid/singular transforms, degenerate normals, reflectance and limits.
    #[allow(clippy::float_cmp)] // Exact affine row; projective transforms are rejected.
    pub fn from_scene_mesh(
        device: &wgpu::Device,
        mesh: &crate::SceneMesh,
        model: glam::Mat4,
        metallic: f32,
        roughness: f32,
    ) -> Result<Self, ReconstructionMaterialError> {
        let determinant = model.determinant();
        if !model.is_finite()
            || !determinant.is_finite()
            || determinant == 0.0
            || model.x_axis.w != 0.0
            || model.y_axis.w != 0.0
            || model.z_axis.w != 0.0
            || model.w_axis.w != 1.0
        {
            return Err(ReconstructionMaterialError);
        }
        let normal_matrix = model.inverse().transpose();
        if !normal_matrix.is_finite() {
            return Err(ReconstructionMaterialError);
        }
        let bytes = mesh
            .indices()
            .len()
            .checked_mul(std::mem::size_of::<ReconstructionGuideVertex>())
            .ok_or(ReconstructionMaterialError)?;
        if u64::try_from(bytes).map_err(|_| ReconstructionMaterialError)?
            > device.limits().max_buffer_size
        {
            return Err(ReconstructionMaterialError);
        }
        let normals = crate::scene::smooth_normals(mesh);
        let vertices = mesh
            .indices()
            .iter()
            .map(|index| {
                let index = usize::try_from(*index).map_err(|_| ReconstructionMaterialError)?;
                let vertex = mesh
                    .vertices()
                    .get(index)
                    .ok_or(ReconstructionMaterialError)?;
                let normal = normals.get(index).ok_or(ReconstructionMaterialError)?;
                ReconstructionGuideVertex::new(
                    model
                        .transform_point3(glam::Vec3::from_array(vertex.position))
                        .to_array(),
                    normal_matrix
                        .transform_vector3(glam::Vec3::from_array(*normal))
                        .to_array(),
                    [vertex.color[0], vertex.color[1], vertex.color[2]],
                    metallic,
                    roughness,
                )?
                .with_uv(vertex.uv)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Self::upload(device, &vertices)
    }

    /// Upload a world-space triangle list; retain mesh through encoding/submission.
    /// # Errors
    /// Rejects empty/non-triangular or oversized lists and zero-initialized normals.
    pub fn upload(
        device: &wgpu::Device,
        vertices: &[ReconstructionGuideVertex],
    ) -> Result<Self, ReconstructionMaterialError> {
        let count = u32::try_from(vertices.len()).map_err(|_| ReconstructionMaterialError)?;
        if count == 0 || !count.is_multiple_of(3) {
            return Err(ReconstructionMaterialError);
        }
        for vertex in vertices {
            ReconstructionGuideVertex::new(
                vertex.position,
                vertex.normal,
                vertex.base_color,
                vertex.metallic_roughness[0],
                vertex.metallic_roughness[1],
            )?
            .with_uv(vertex.uv)?;
        }
        Ok(Self {
            vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("RR guide triangles"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
            count,
        })
    }
}
#[derive(Debug)]
pub struct ReconstructionGuideInputs {
    bindings: wgpu::BindGroup,
    size: wgpu::Extent3d,
}
#[derive(Debug)]
pub struct ReconstructionGuidePass {
    device: wgpu::Device,
    pipeline: wgpu::RenderPipeline,
    f0_pipeline: wgpu::RenderPipeline,
    object_pipeline: wgpu::RenderPipeline,
    object_layout: wgpu::BindGroupLayout,
    layout: wgpu::BindGroupLayout,
}
impl ReconstructionGuidePass {
    /// Device must support the four-guide formats/MRT limits checked by guide allocation.
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        Self::create(device, wgpu::TextureFormat::R32Float)
    }
    /// Match the scalar attachment selected by guide allocation (R32 or R16).
    #[must_use]
    pub fn for_guides(device: &wgpu::Device, guides: &RayReconstructionGuides) -> Self {
        Self::create(device, guides.specular_hit_distance().format())
    }
    fn create(device: &wgpu::Device, distance_format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("RR guide inputs"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(80),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("RR guide pipeline"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("RR material shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("reconstruction_pass.wgsl").into()),
        });
        let targets = [
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureFormat::Rgba16Float,
            distance_format,
        ]
        .map(|format| {
            Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("RR opaque material guides"), layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), compilation_options: wgpu::PipelineCompilationOptions::default(), buffers: &[Some(wgpu::VertexBufferLayout { array_stride: 52, step_mode: wgpu::VertexStepMode::Vertex, attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32x2, 4 => Float32x2] })] },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs_main"), compilation_options: wgpu::PipelineCompilationOptions::default(), targets: &targets }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState { format: wgpu::TextureFormat::Depth32Float, depth_write_enabled: Some(true), depth_compare: Some(wgpu::CompareFunction::LessEqual), stencil: wgpu::StencilState::default(), bias: wgpu::DepthBiasState::default() }),
            multisample: wgpu::MultisampleState::default(), multiview_mask: None, cache: None,
        });
        let f0_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("primary material F0"), layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), compilation_options: wgpu::PipelineCompilationOptions::default(), buffers: &[Some(wgpu::VertexBufferLayout { array_stride: 52, step_mode: wgpu::VertexStepMode::Vertex, attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32x2, 4 => Float32x2] })] },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs_f0"), compilation_options: wgpu::PipelineCompilationOptions::default(), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba16Float, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState { format: wgpu::TextureFormat::Depth32Float, depth_write_enabled: Some(false), depth_compare: Some(wgpu::CompareFunction::Equal), stencil: wgpu::StencilState::default(), bias: wgpu::DepthBiasState::default() }),
            multisample: wgpu::MultisampleState::default(), multiview_mask: None, cache: None,
        });
        let (object_pipeline, object_layout) = object_pipeline(device, &layout, &shader);
        Self {
            device: device.clone(),
            pipeline,
            f0_pipeline,
            object_pipeline,
            object_layout,
            layout,
        }
    }
    /// Bind the current camera and ray-traced `R32Float` specular distances.
    /// Distances must match guide dimensions, primary surfaces and this camera;
    /// source must be distinct from the output hit-distance texture.
    /// # Errors
    /// Rejects invalid camera, hit-distance format/usage/dimensions.
    pub fn inputs(
        &self,
        device: &wgpu::Device,
        view_projection: glam::Mat4,
        camera_position: [f32; 3],
        hit_distance: &wgpu::Texture,
    ) -> Result<ReconstructionGuideInputs, ReconstructionMaterialError> {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("unused guide base color"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        self.material_inputs(
            device,
            view_projection,
            camera_position,
            hit_distance,
            &texture,
            &sampler,
            false,
        )
    }
    /// Bind a base-color texture for the material guide passes. RGB multiplies
    /// vertex color; sRGB views decode to linear. Alpha is ignored (opaque path).
    /// Use a noncomparison sampler on this device; guide camera/mesh UVs must match.
    /// # Errors
    /// Rejects unsupported texture descriptors and existing camera/distance errors.
    #[allow(clippy::too_many_arguments)]
    pub fn textured_inputs(
        &self,
        device: &wgpu::Device,
        view_projection: glam::Mat4,
        camera_position: [f32; 3],
        hit_distance: &wgpu::Texture,
        base_color: &wgpu::Texture,
        sampler: &wgpu::Sampler,
    ) -> Result<ReconstructionGuideInputs, ReconstructionMaterialError> {
        self.material_inputs(
            device,
            view_projection,
            camera_position,
            hit_distance,
            base_color,
            sampler,
            true,
        )
    }
    /// Reuse a scene renderer's uploaded material without reuploading image data.
    /// Camera, device, UV and opaque-surface contracts match `textured_inputs`.
    /// # Errors
    /// Preserves texture and camera/distance validation errors.
    pub fn scene_material_inputs(
        &self,
        device: &wgpu::Device,
        view_projection: glam::Mat4,
        camera_position: [f32; 3],
        hit_distance: &wgpu::Texture,
        material: &crate::SceneTexture,
    ) -> Result<ReconstructionGuideInputs, ReconstructionMaterialError> {
        if !material.belongs_to(device) {
            return Err(ReconstructionMaterialError);
        }
        self.textured_inputs(
            device,
            view_projection,
            camera_position,
            hit_distance,
            material.texture(),
            material.sampler(),
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn material_inputs(
        &self,
        device: &wgpu::Device,
        view_projection: glam::Mat4,
        camera_position: [f32; 3],
        hit_distance: &wgpu::Texture,
        base_color: &wgpu::Texture,
        sampler: &wgpu::Sampler,
        textured: bool,
    ) -> Result<ReconstructionGuideInputs, ReconstructionMaterialError> {
        if device != &self.device {
            return Err(ReconstructionMaterialError);
        }
        if !matches!(
            base_color.format(),
            wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb
        ) || base_color.dimension() != wgpu::TextureDimension::D2
            || base_color.sample_count() != 1
            || base_color.depth_or_array_layers() != 1
            || !base_color
                .usage()
                .contains(wgpu::TextureUsages::TEXTURE_BINDING)
        {
            return Err(ReconstructionMaterialError);
        }
        if !view_projection.is_finite()
            || !view_projection.inverse().is_finite()
            || !glam::Vec3::from_array(camera_position).is_finite()
            || hit_distance.format() != wgpu::TextureFormat::R32Float
            || hit_distance.sample_count() != 1
            || hit_distance.depth_or_array_layers() != 1
            || hit_distance.dimension() != wgpu::TextureDimension::D2
            || !hit_distance
                .usage()
                .contains(wgpu::TextureUsages::TEXTURE_BINDING)
        {
            return Err(ReconstructionMaterialError);
        }
        let mut data = view_projection.to_cols_array().to_vec();
        data.extend_from_slice(&[
            camera_position[0],
            camera_position[1],
            camera_position[2],
            if textured { 1.0 } else { 0.0 },
        ]);
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("RR guide camera"),
            contents: bytemuck::cast_slice(&data),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let view = hit_distance.create_view(&wgpu::TextureViewDescriptor::default());
        let material_view = base_color.create_view(&wgpu::TextureViewDescriptor::default());
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("RR guide bindings"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&material_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        Ok(ReconstructionGuideInputs {
            bindings,
            size: hit_distance.size(),
        })
    }
    /// Build camera inputs for the primary guide pass before reflection tracing.
    /// Starts hit distance at zero using wgpu's zero-initialized texture contents.
    /// After primary reconstruction/tracing, update it with `ReconstructionDistancePass`
    /// before RR evaluation. These inputs own the placeholder through their bindings.
    /// # Errors
    /// Rejects invalid camera and dimensions unsupported by this device.
    pub fn primary_inputs(
        &self,
        device: &wgpu::Device,
        view_projection: glam::Mat4,
        camera_position: [f32; 3],
        guides: &RayReconstructionGuides,
    ) -> Result<ReconstructionGuideInputs, ReconstructionMaterialError> {
        let size = guides.size();
        if size.width > device.limits().max_texture_dimension_2d
            || size.height > device.limits().max_texture_dimension_2d
        {
            return Err(ReconstructionMaterialError);
        }
        let distance = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("untraced primary distance placeholder"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        self.inputs(device, view_projection, camera_position, &distance)
    }
    /// Rasterize linear F0 only at the exact opaque depth written by `encode`.
    /// Call after the primary guide pass with the same camera and meshes. This
    /// separate attachment preserves the four-guide MRT byte limit.
    /// # Errors
    /// Rejects guide/input size mismatch; native attachment validation is by wgpu.
    pub fn encode_material_f0(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        guides: &RayReconstructionGuides,
        depth: &wgpu::TextureView,
        inputs: &ReconstructionGuideInputs,
        meshes: &[&ReconstructionGuideMesh],
    ) -> Result<(), ReconstructionMaterialError> {
        let draws: Vec<_> = meshes.iter().map(|mesh| (*mesh, inputs)).collect();
        self.encode_material_f0_draws(encoder, guides, depth, &draws)
    }
    /// Draw independent material inputs per mesh in one attachment pass.
    /// Inputs must share camera, world coordinates, device and guide dimensions.
    /// # Errors
    /// Rejects any size mismatch before clearing or encoding attachments.
    pub fn encode_material_f0_draws(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        guides: &RayReconstructionGuides,
        depth: &wgpu::TextureView,
        draws: &[(&ReconstructionGuideMesh, &ReconstructionGuideInputs)],
    ) -> Result<(), ReconstructionMaterialError> {
        if draws.iter().any(|(_, inputs)| guides.size() != inputs.size) {
            return Err(ReconstructionMaterialError);
        }
        let view = guides
            .material_f0()
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("primary F0 at opaque depth"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
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
        pass.set_pipeline(&self.f0_pipeline);
        for (mesh, inputs) in draws {
            pass.set_bind_group(0, &inputs.bindings, &[]);
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.draw(0..mesh.count, 0..1);
        }
        Ok(())
    }
    /// Rasterize object IDs only at the matching opaque primary depth.
    /// Use the same meshes/camera as `encode`; background is `u32::MAX`.
    /// ID maps are separate attachments and do not change the four-guide MRT limit.
    /// # Errors
    /// Rejects mismatched dimensions and the reserved background ID.
    pub fn encode_object_ids(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        guides: &RayReconstructionGuides,
        depth: &wgpu::TextureView,
        inputs: &ReconstructionGuideInputs,
        meshes: &[(&ReconstructionGuideMesh, u32)],
    ) -> Result<(), ReconstructionMaterialError> {
        if guides.size() != inputs.size || meshes.iter().any(|(_, id)| *id == u32::MAX) {
            return Err(ReconstructionMaterialError);
        }
        let bindings: Vec<_> = meshes
            .iter()
            .map(|(_, id)| {
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("primary object identity"),
                    contents: bytemuck::cast_slice(&[*id, 0, 0, 0]),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("primary object identity"),
                    layout: &self.object_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                })
            })
            .collect();
        let view = guides
            .object_ids()
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("primary object IDs at opaque depth"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: f64::from(u32::MAX),
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    }),
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
        pass.set_pipeline(&self.object_pipeline);
        pass.set_bind_group(0, &inputs.bindings, &[]);
        for ((mesh, _), binding) in meshes.iter().zip(&bindings) {
            pass.set_bind_group(1, binding, &[]);
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.draw(0..mesh.count, 0..1);
        }
        Ok(())
    }
    /// Clear guides/depth then draw opaque triangle lists. Depth must be same-size
    /// single-sample `Depth32Float`, on this device. Exclude UI/transparent geometry.
    /// # Errors
    /// Rejects ray input and guide size mismatch; wgpu validates native attachments.
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        guides: &RayReconstructionGuides,
        depth: &wgpu::TextureView,
        inputs: &ReconstructionGuideInputs,
        meshes: &[&ReconstructionGuideMesh],
    ) -> Result<(), ReconstructionMaterialError> {
        let draws: Vec<_> = meshes.iter().map(|mesh| (*mesh, inputs)).collect();
        self.encode_draws(encoder, guides, depth, &draws)
    }
    /// Draw independent material inputs per mesh in one attachment pass.
    /// Inputs must share camera, world coordinates, device and guide dimensions.
    /// # Errors
    /// Rejects any size mismatch before clearing or encoding attachments.
    pub fn encode_draws(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        guides: &RayReconstructionGuides,
        depth: &wgpu::TextureView,
        draws: &[(&ReconstructionGuideMesh, &ReconstructionGuideInputs)],
    ) -> Result<(), ReconstructionMaterialError> {
        if draws.iter().any(|(_, inputs)| guides.size() != inputs.size) {
            return Err(ReconstructionMaterialError);
        }
        let views = [
            guides.normal_roughness(),
            guides.diffuse_albedo(),
            guides.specular_albedo(),
            guides.specular_hit_distance(),
        ]
        .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
        let attachments = views.each_ref().map(|view| {
            Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("RR material guide pass"),
            color_attachments: &attachments,
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        for (mesh, inputs) in draws {
            pass.set_bind_group(0, &inputs.bindings, &[]);
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.draw(0..mesh.count, 0..1);
        }
        Ok(())
    }
}

fn object_pipeline(
    device: &wgpu::Device,
    camera_layout: &wgpu::BindGroupLayout,
    shader: &wgpu::ShaderModule,
) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("primary object ID layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(16),
            },
            count: None,
        }],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("primary object ID pipeline layout"),
        bind_group_layouts: &[Some(camera_layout), Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("primary object identity"), layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: shader, entry_point: Some("vs_main"), compilation_options: wgpu::PipelineCompilationOptions::default(), buffers: &[Some(wgpu::VertexBufferLayout { array_stride: 52, step_mode: wgpu::VertexStepMode::Vertex, attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32x2, 4 => Float32x2] })] },
            fragment: Some(wgpu::FragmentState { module: shader, entry_point: Some("fs_object"), compilation_options: wgpu::PipelineCompilationOptions::default(), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::R32Uint, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState { format: wgpu::TextureFormat::Depth32Float, depth_write_enabled: Some(false), depth_compare: Some(wgpu::CompareFunction::Equal), stencil: wgpu::StencilState::default(), bias: wgpu::DepthBiasState::default() }),
            multisample: wgpu::MultisampleState::default(), multiview_mask: None, cache: None,
        });
    (pipeline, layout)
}
